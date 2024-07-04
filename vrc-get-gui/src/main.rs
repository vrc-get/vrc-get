// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

mod commands;
mod config;
mod deep_link_support;
mod logging;
mod specta;
mod templates;

#[cfg_attr(windows, path = "os_windows.rs")]
#[cfg_attr(not(windows), path = "os_posix.rs")]
mod os;

// for clippy compatibility
#[cfg(not(clippy))]
fn tauri_context() -> tauri::Context<impl tauri::Assets> {
    tauri::generate_context!()
}

#[cfg(clippy)]
fn tauri_context() -> tauri::Context<tauri::utils::assets::EmbeddedAssets> {
    panic!()
}

fn main() {
    let io = logging::initialize_logger();

    // logger is now initialized, we can use log for panics
    log_panics::init();

    #[cfg(debug_assertions)]
    commands::export_ts();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            log::info!("single instance remote procedure, {argv:?}, {cwd}");
            if let Some(window) = app.get_window("main") {
                if let Err(e) = window.unminimize() {
                    log::error!("error while unminimize: {}", e);
                }
                if let Err(e) = window.set_focus() {
                    log::error!("error while setting focus: {}", e);
                }
            }
            process_args(&argv);
        }))
        .invoke_handler(commands::handlers())
        .setup(move |app| {
            app.manage(commands::new_env_state(io));
            commands::startup(app);
            // process args
            process_args(&std::env::args().collect::<Vec<_>>());
            Ok(())
        })
        .build(tauri_context())
        .expect("error while building tauri application");

    // deep link support
    #[cfg(target_os = "macos")]
    objc_patch::patch_delegate();
    deep_link_support::set_app_handle(app.handle());

    logging::set_app_handle(app.handle());
    app.run(|_, _| {})
}

fn process_args(args: &[String]) {
    if args.len() <= 1 {
        // no additional args
        return;
    }

    if args.len() == 2 {
        // we have a single argument. it might be a deep link
        let arg = &args[1];
        if arg.starts_with("vcc://") {
            process_deep_link_string(arg);
        }
    }

    match args[1].as_str() {
        "link" => {
            let Some(url) = args.get(2) else {
                log::error!("link command requires a URL argument");
                return;
            };
            process_deep_link_string(url);
        }
        _ => {
            log::error!("Unknown command: {}", args[1]);
        }
    }

    fn process_deep_link_string(url: &str) {
        match url::Url::parse(url) {
            Ok(url) => {
                deep_link_support::on_deep_link(url);
            }
            Err(e) => {
                log::error!("Failed to parse deep link: {}", e);
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod objc_patch {
    use cocoa::base::{id, nil};
    use cocoa::foundation::{NSArray, NSAutoreleasePool, NSString, NSURL};
    use objc::declare::MethodImplementation;
    use objc::runtime::*;
    use objc::*;
    use std::ffi::CString;

    pub(crate) fn patch_delegate() {
        unsafe {
            // Note: this patch is heavily depending on tao's internal implementation
            let delegate_class = class!(TaoAppDelegate);
            add_method(
                delegate_class,
                sel!(applicationWillFinishLaunching:),
                will_finish_launching as extern "C" fn(&Object, Sel, id),
            );
            log::debug!("applicationWillFinishLaunching patched");

            add_method(
                delegate_class,
                sel!(application:openURLs:),
                open_urls as extern "C" fn(&Object, Sel, id, id),
            );
            log::debug!("application:openURLs: patched");

            // to enable the patch, we need to re-assign the delegate

            let app_class = class!(TaoApp);
            let app: id = msg_send![app_class, sharedApplication];

            let pool = NSAutoreleasePool::new(nil);

            let delegate: id = msg_send![app, delegate];
            let _: () = msg_send![app, setDelegate: nil];
            let _: () = msg_send![app, setDelegate: delegate];

            let _: () = msg_send![pool, drain];
        }
    }

    extern "C" fn will_finish_launching(_: &Object, _: Sel, _: id) {
        log::debug!("applicationWillFinishLaunching:")
    }

    extern "C" fn open_urls(_: &Object, _: Sel, _application: id, urls: id) {
        log::debug!("application:openURLs:");

        let urls = unsafe {
            (0..urls.count())
                .flat_map(|i| {
                    let string = urls.objectAtIndex(i).absoluteString();
                    let as_slice =
                        std::slice::from_raw_parts(string.UTF8String() as *const u8, string.len());
                    let as_str = std::str::from_utf8(as_slice).ok()?;
                    url::Url::parse(as_str).ok()
                })
                .collect::<Vec<_>>()
        };
        for x in urls {
            log::debug!("URL: {x}");
            if x.scheme() == "vcc" {
                crate::deep_link_support::on_deep_link(x);
            }
        }
    }

    //region adding method implementation
    unsafe fn add_method<F>(class: &Class, sel: Sel, func: F)
    where
        F: MethodImplementation<Callee = Object>,
    {
        let encs = F::Args::encodings();
        let encs = encs.as_ref();
        let sel_args = count_args(sel);
        assert!(
            sel_args == encs.len(),
            "Selector accepts {} arguments, but function accepts {}",
            sel_args,
            encs.len(),
        );

        let types = method_type_encoding(&F::Ret::encode(), encs);
        let success = class_addMethod(class as *const _ as *mut _, sel, func.imp(), types.as_ptr());
        assert!(success != NO, "Failed to add method {:?}", sel);
    }

    fn count_args(sel: Sel) -> usize {
        sel.name().chars().filter(|&c| c == ':').count()
    }

    fn method_type_encoding(ret: &Encoding, args: &[Encoding]) -> CString {
        let mut types = ret.as_str().to_owned();
        // First two arguments are always self and the selector
        types.push_str(<*mut Object>::encode().as_str());
        types.push_str(Sel::encode().as_str());
        types.extend(args.iter().map(|e| e.as_str()));
        CString::new(types).unwrap()
    }
    // endregion
}

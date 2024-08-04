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
mod state;
mod utils;

// for clippy compatibility
#[cfg(not(clippy))]
fn tauri_context() -> tauri::Context {
    tauri::generate_context!()
}

#[cfg(clippy)]
fn tauri_context() -> tauri::Context {
    panic!()
}

fn main() {
    let io = logging::initialize_logger();

    // logger is now initialized, we can use log for panics
    log_panics::init();

    #[cfg(dev)]
    commands::export_ts();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            log::info!("single instance remote procedure, {argv:?}, {cwd}");
            if let Some(window) = app.get_webview_window("main") {
                if let Err(e) = window.unminimize() {
                    log::error!("error while unminimize: {}", e);
                }
                if let Err(e) = window.set_focus() {
                    log::error!("error while setting focus: {}", e);
                }
            }
            process_args(&argv);
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(io.clone())
        .manage(state::new_http_client())
        .manage(state::SettingsState::new())
        .manage(state::UpdaterState::new())
        .manage(state::ProjectsState::new())
        .manage(state::PackagesState::new())
        .register_uri_scheme_protocol("vrc-get", commands::handle_vrc_get_scheme)
        .invoke_handler(commands::handlers())
        .setup(move |app| {
            commands::startup(app);
            // process args
            process_args(&std::env::args().collect::<Vec<_>>());
            Ok(())
        })
        .build(tauri_context())
        .expect("error while building tauri application");

    os::initialize(app.handle().clone());

    deep_link_support::set_app_handle(app.handle().clone());

    logging::set_app_handle(app.handle().clone());
    app.run(|_, event| match event {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        tauri::RunEvent::Opened { urls } => {
            for url in urls {
                deep_link_support::on_deep_link(url);
            }
        }
        _ => {}
    })
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

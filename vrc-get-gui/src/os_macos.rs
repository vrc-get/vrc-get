// macOS-specific functionality.

use cocoa::foundation::NSProcessInfo;
use dispatch2::Queue;
use objc2::__framework_prelude::Retained;
use objc2::AllocAnyThread;
use objc2_app_kit::{NSRunningApplication, NSWorkspace, NSWorkspaceOpenConfiguration};
use objc2_foundation::{NSError, NSString, NSURL};
use std::ffi::OsStr;
use std::io;
use std::io::Cursor;
use std::ops::DerefMut;
use std::path::{Component, Path};
use std::pin::Pin;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

/// On macOS, we try opening with NSWorkspace openApplicationAtURL:configuration:completionHandler:
/// if the path is executable of the application bundle, and fallback to start_command_posix if not
pub(crate) async fn start_command(name: &OsStr, path: &OsStr, args: &[&OsStr]) -> io::Result<()> {
    return if let (Some(app_bundle_path), Some(args)) = (
        app_bundle_path(path).await,
        args.iter()
            .map(|arg| arg.to_str())
            .collect::<Option<Vec<&str>>>(),
    ) {
        // The path is executable of an app bundle.
        log::info!("launch_new_app_with_ns_workspace: {app_bundle_path:?}");
        launch_new_app_with_ns_workspace(app_bundle_path, &args).await
    } else {
        // The path is not executable of an app bundle.
        // We use normal app part.
        super::start_command_posix(name, path, args).await
    };

    /// Check if the path ends with `AppName.app/Contents/MacOS/Executable` and the `Executable` is
    /// the same as `CFBundleExecutable` of `AppName.app/Contents/Info.plist`
    async fn app_bundle_path(path: &OsStr) -> Option<&str> {
        let path = Path::new(path);
        let mut path_components = path.components();
        // We get components in reverse order with next_back();
        let Component::Normal(app_executable_name) = path_components.next_back()? else {
            return None;
        };
        let macos = path_components.next_back()?;
        let contents = path_components.next_back()?;
        let app_bundle_path = path_components.as_path();

        if macos != Component::Normal("MacOS".as_ref()) {
            return None;
        }
        if contents != Component::Normal("Contents".as_ref()) {
            return None;
        }
        if app_bundle_path.extension() != Some(OsStr::new("app")) {
            return None;
        }

        // We now confined the path ends with `AppName.app/Contents/MacOS/Executable`.
        // so we read `AppName.app/Contents/Info.plist`
        // and check if `CFBundleExecutable` value is the same as Executable
        let plist_path = app_bundle_path.join("Contents/Info.plist");
        let plist_file = tokio::fs::read(&plist_path).await.ok()?;
        let plist = plist::Value::from_reader(Cursor::new(plist_file.as_slice())).ok()?;
        let bundle_executable = plist
            .as_dictionary()
            .and_then(|x| x.get("CFBundleExecutable"))
            .and_then(|x| x.as_string())?;

        if app_executable_name != bundle_executable {
            return None;
        }

        app_bundle_path.as_os_str().to_str()
    }
}

fn launch_new_app_with_ns_workspace(
    app_bundle_path: &str,
    args: &[&str],
) -> impl Future<Output = io::Result<()>> {
    struct FutureImpl {
        state: Arc<Mutex<State>>,
    }
    enum State {
        Initial(String, Vec<String>),
        Pending(std::task::Waker),
        WithResult(io::Result<()>),
        Finished,
    }

    impl Future for FutureImpl {
        type Output = io::Result<()>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut state = self.state.lock().unwrap();
            match state.deref_mut() {
                State::Initial(_, _) => {
                    // It was the initial state. Move to pending and dispatch
                    let State::Initial(app_bundle_path, args) =
                        std::mem::replace(&mut *state, State::Pending(cx.waker().clone()))
                    else {
                        unreachable!()
                    };
                    let state = self.state.clone();
                    do_launch(app_bundle_path, args, move |result| {
                        let mut state = state.lock().unwrap();
                        let State::Pending(_) = &*state else {
                            unreachable!();
                        };
                        let State::Pending(waker) =
                            std::mem::replace(state.deref_mut(), State::WithResult(result))
                        else {
                            unreachable!();
                        };

                        waker.wake();
                    });
                    Poll::Pending
                }
                State::Pending(waker) => {
                    *waker = cx.waker().clone();
                    Poll::Pending
                }
                State::WithResult(_) => {
                    let State::WithResult(result) = std::mem::replace(&mut *state, State::Finished)
                    else {
                        unreachable!()
                    };
                    Poll::Ready(result)
                }
                State::Finished => {
                    panic!("future polled after completion");
                }
            }
        }
    }

    fn do_launch(
        app_bundle_path: String,
        args: Vec<String>,
        on_result: impl Fn(io::Result<()>) + Send + Sync + 'static,
    ) {
        Queue::main().exec_async(move || unsafe {
            let configuration = NSWorkspaceOpenConfiguration::new();

            configuration.setCreatesNewApplicationInstance(true);
            configuration.setArguments(
                &args
                    .iter()
                    .map(|arg| NSString::from_str(arg))
                    .collect::<Retained<_>>(),
            );

            NSWorkspace::sharedWorkspace().openApplicationAtURL_configuration_completionHandler(
                &NSURL::initFileURLWithPath(NSURL::alloc(), &NSString::from_str(&app_bundle_path)),
                &configuration,
                Some(&*block2::RcBlock::new(
                    move |_: *mut NSRunningApplication, err: *mut NSError| {
                        let result = if !err.is_null() {
                            Err(io::Error::other(&*err))
                        } else {
                            Ok(())
                        };

                        on_result(result);
                    },
                )),
            );
        });
    }

    // NSString is thread safe so copy to NSString here first
    FutureImpl {
        state: Arc::new(Mutex::new(State::Initial(
            app_bundle_path.into(),
            args.iter().cloned().map(Into::into).collect(),
        ))),
    }
}

pub(super) fn compute_os_info() -> String {
    unsafe {
        let process_info = NSProcessInfo::processInfo(std::ptr::null_mut());
        let os_info = NSProcessInfo::operatingSystemVersion(process_info);

        format!(
            "macOS {}.{}.{}",
            os_info.majorVersion, os_info.minorVersion, os_info.patchVersion,
        )
    }
}

pub use open::that as open_that;

pub fn initialize(_: tauri::AppHandle) {
    // nothing to initialize
}

pub(crate) fn fix_env_variables(_: &mut Command) {
    // nothing to do
}

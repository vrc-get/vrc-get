use crate::commands::prelude::*;

use log::{error, info};
use std::io;
use tauri::async_runtime::spawn;
use tauri::{App, AppHandle, LogicalSize, Manager, State, Window, WindowEvent};
use tokio::sync::Mutex;
use vrc_get_vpm::unity_hub;

trait WindowExt {
    fn make_fullscreen_ish(&self) -> tauri::Result<()>;
    fn is_fullscreen_ish(&self) -> tauri::Result<bool>;
}

impl WindowExt for Window {
    fn make_fullscreen_ish(&self) -> tauri::Result<()> {
        if cfg!(windows) {
            self.maximize()
        } else {
            self.set_fullscreen(true)
        }
    }

    fn is_fullscreen_ish(&self) -> tauri::Result<bool> {
        if cfg!(windows) {
            self.is_maximized()
        } else {
            self.is_fullscreen()
        }
    }
}
pub fn startup(app: &mut App) {
    let handle = app.handle();
    tauri::async_runtime::spawn(async move {
        let state = handle.state();
        if let Err(e) = update_unity_hub(state).await {
            error!("failed to update unity from unity hub: {e}");
        }
    });

    let handle = app.handle();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = open_main(handle).await {
            error!("failed to open main window: {e}");
        }
    });

    async fn update_unity_hub(state: State<'_, Mutex<EnvironmentState>>) -> Result<(), io::Error> {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let unity_hub_path = with_environment!(&state, |environment| {
            let Some(unity_hub_path) = environment.find_unity_hub().await? else {
                error!("Unity Hub not found");
                return Ok(());
            };
            environment.save().await?;
            unity_hub_path
        });

        let paths_from_hub = unity_hub::get_unity_from_unity_hub(unity_hub_path.as_ref()).await?;

        with_environment!(&state, |environment| {
            environment
                .update_unity_from_unity_hub_and_fs(&paths_from_hub)
                .await?;

            environment.save().await?;
        });

        info!("finished updating unity from unity hub");
        Ok(())
    }

    async fn open_main(app: AppHandle) -> tauri::Result<()> {
        let state: State<'_, Mutex<EnvironmentState>> = app.state();
        let config = with_config!(state, |config| config.clone());

        if !cfg!(target_os = "macos") && config.use_alcom_for_vcc_protocol {
            spawn(crate::deep_link_support::deep_link_install_vcc(app.clone()));
        }

        let query = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("lang", &config.language)
            .append_pair("theme", &config.theme)
            .finish();

        use super::environment::config::SetupPages;
        let start_page = SetupPages::pages()
            .iter()
            .copied()
            .filter(|page| !page.is_finished(config.setup_process_progress))
            .next()
            .map(|x| x.path())
            .unwrap_or("/projects/");

        let window = tauri::WindowBuilder::new(
            &app,
            "main", /* the unique window label */
            tauri::WindowUrl::App(format!("{start_page}?{query}").into()),
        )
        .title("ALCOM")
        .resizable(true)
        .on_navigation(|url| {
            if cfg!(debug_assertions) {
                url.host_str() == Some("localhost")
            } else if cfg!(windows) {
                url.scheme() == "https" && url.host_str() == Some("tauri.localhost")
            } else {
                url.scheme() == "tauri"
            }
        })
        .build()?;

        // keep original size if it's too small
        if config.window_size.width > 100 && config.window_size.height > 100 {
            window.set_size(LogicalSize {
                width: config.window_size.width,
                height: config.window_size.height,
            })?;
        }

        if config.fullscreen {
            window.make_fullscreen_ish()?;
        }

        let cloned = window.clone();

        let resize_debounce: std::sync::Mutex<Option<tauri::async_runtime::JoinHandle<()>>> =
            std::sync::Mutex::new(None);

        #[allow(clippy::single_match)]
        window.on_window_event(move |e| match e {
            WindowEvent::Resized(size) => {
                let logical = size
                    .to_logical::<u32>(cloned.current_monitor().unwrap().unwrap().scale_factor());

                if logical.width < 100 || logical.height < 100 {
                    // ignore too small sizes
                    // this is generally caused by the window being minimized
                    return;
                }

                let fullscreen = cloned.is_fullscreen_ish().unwrap();

                let mut resize_debounce = resize_debounce.lock().unwrap();

                if let Some(resize_debounce) = resize_debounce.as_ref() {
                    resize_debounce.abort();
                }

                let cloned = cloned.clone();

                *resize_debounce = Some(tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                    if let Err(e) = save_window_size(cloned.state(), logical, fullscreen).await {
                        error!("failed to save window size: {e}");
                    }
                }));
            }
            _ => {}
        });

        async fn save_window_size(
            state: State<'_, Mutex<EnvironmentState>>,
            size: LogicalSize<u32>,
            fullscreen: bool,
        ) -> tauri::Result<()> {
            info!(
                "saving window size: {}x{}, full: {}",
                size.width, size.height, fullscreen
            );
            with_config!(state, |mut config| {
                if fullscreen {
                    config.fullscreen = true;
                } else {
                    config.fullscreen = false;
                    config.window_size.width = size.width;
                    config.window_size.height = size.height;
                }
                config.save().await?;
            });
            Ok(())
        }

        Ok(())
    }
}

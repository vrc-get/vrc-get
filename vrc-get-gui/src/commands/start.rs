use crate::commands::prelude::*;

use crate::commands::environment::unity_hub::update_unity_paths_from_unity_hub;
use log::{error, info};
use std::io;
use tauri::async_runtime::spawn;
use tauri::{App, AppHandle, LogicalSize, Manager, State, WebviewWindow, WindowEvent};
use vrc_get_vpm::io::DefaultEnvironmentIo;

trait WindowExt {
    fn make_fullscreen_ish(&self) -> tauri::Result<()>;
    fn is_fullscreen_ish(&self) -> tauri::Result<bool>;
}

impl WindowExt for WebviewWindow {
    fn make_fullscreen_ish(&self) -> tauri::Result<()> {
        if !cfg!(target_os = "macos") {
            self.maximize()
        } else {
            self.set_fullscreen(true)
        }
    }

    fn is_fullscreen_ish(&self) -> tauri::Result<bool> {
        if !cfg!(target_os = "macos") {
            self.is_maximized()
        } else {
            self.is_fullscreen()
        }
    }
}
pub fn startup(app: &mut App) {
    let handle = app.handle().clone();
    spawn(async move {
        if let Err(e) = open_main(handle).await {
            error!("failed to open main window: {e}");
        }
    });

    async fn update_unity_hub(
        settings: State<'_, SettingsState>,
        config: State<'_, GuiConfigState>,
        io: State<'_, DefaultEnvironmentIo>,
    ) -> Result<(), io::Error> {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        if update_unity_paths_from_unity_hub(&settings, &config, &io).await? {
            info!("finished updating unity from unity hub");
        } else {
            error!("Unity Hub not found");
        }

        Ok(())
    }

    async fn open_main(app: AppHandle) -> tauri::Result<()> {
        let io = app.state::<DefaultEnvironmentIo>();
        let config = GuiConfigState::new_load(io.inner()).await?;
        app.manage(config);

        let handle = app.clone();
        spawn(async move {
            let state = handle.state();
            let config = handle.state();
            let io = handle.state();
            if let Err(e) = update_unity_hub(state, config, io).await {
                error!("failed to update unity from unity hub: {e}");
            }
        });

        let config = app.state::<GuiConfigState>();
        let config = config.get().clone();

        if crate::deep_link_support::should_install_deep_link(&app)
            && config.use_alcom_for_vcc_protocol
        {
            spawn(crate::deep_link_support::deep_link_install_vcc(app.clone()));
        }

        use super::environment::config::SetupPages;
        let start_page = SetupPages::pages(&app)
            .iter()
            .copied()
            .find(|page| !page.is_finished(config.setup_process_progress))
            .map(|x| x.path())
            .unwrap_or("/projects/");

        let window = tauri::WebviewWindowBuilder::new(
            &app,
            "main", /* the unique window label */
            tauri::WebviewUrl::App(start_page.into()),
        )
        .title("ALCOM")
        .resizable(true)
        .incognito(true) // this prevents the webview from saving data
        .on_navigation(|url| {
            if cfg!(debug_assertions) && url.host_str() == Some("localhost") {
                return true;
            }
            if cfg!(windows) {
                url.scheme() == "http" && url.host_str() == Some("tauri.localhost")
                    || url.host_str() == Some("vrc-get.localhost")
            } else {
                url.scheme() == "tauri" || url.scheme() == "vrc-get"
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

                    if let Err(e) = save_window_size(cloned, logical, fullscreen).await {
                        error!("failed to save window size: {e}");
                    }
                }));
            }
            _ => {}
        });

        async fn save_window_size(
            window: WebviewWindow,
            size: LogicalSize<u32>,
            fullscreen: bool,
        ) -> tauri::Result<()> {
            info!(
                "saving window size: {}x{}, full: {}",
                size.width, size.height, fullscreen
            );
            let config = window.state::<GuiConfigState>();
            let mut config = config.load_mut().await?;
            if fullscreen {
                config.fullscreen = true;
            } else {
                config.fullscreen = false;
                config.window_size.width = size.width;
                config.window_size.height = size.height;
            }
            config.save().await?;
            Ok(())
        }

        Ok(())
    }
}

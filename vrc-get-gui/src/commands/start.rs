use crate::commands::prelude::*;

use log::{error, info};
use std::io;
use tauri::async_runtime::spawn;
use tauri::{App, AppHandle, LogicalSize, Manager, State, WebviewWindow, WindowEvent};
use vrc_get_vpm::environment::{find_unity_hub, VccDatabaseConnection};
use vrc_get_vpm::io::DefaultEnvironmentIo;
use vrc_get_vpm::unity_hub;

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
    tauri::async_runtime::spawn(async move {
        let state = handle.state();
        let io = handle.state();
        if let Err(e) = update_unity_hub(state, io).await {
            error!("failed to update unity from unity hub: {e}");
        }
    });

    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = open_main(handle).await {
            error!("failed to open main window: {e}");
        }
    });

    async fn update_unity_hub(
        settings: State<'_, SettingsState>,
        io: State<'_, DefaultEnvironmentIo>,
    ) -> Result<(), io::Error> {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let unity_hub_path = {
            let mut settings = settings.load_mut(io.inner()).await?;
            let Some(unity_hub_path) = find_unity_hub(&mut settings, io.inner()).await? else {
                error!("Unity Hub not found");
                settings.save().await?;
                return Ok(());
            };
            settings.save().await?;
            unity_hub_path
        };

        let paths_from_hub = unity_hub::get_unity_from_unity_hub(unity_hub_path.as_ref()).await?;

        {
            let mut connection = VccDatabaseConnection::connect(io.inner())?;

            connection
                .update_unity_from_unity_hub_and_fs(&paths_from_hub, io.inner())
                .await?;

            connection.save(io.inner()).await?;
        }

        info!("finished updating unity from unity hub");
        Ok(())
    }

    async fn open_main(app: AppHandle) -> tauri::Result<()> {
        let io = app.state::<DefaultEnvironmentIo>();
        let config = GuiConfigState::new_load(io.inner()).await?;
        app.manage(config);

        let config = app.state::<GuiConfigState>();
        let config = config.get().clone();

        if !cfg!(target_os = "macos") && config.use_alcom_for_vcc_protocol {
            spawn(crate::deep_link_support::deep_link_install_vcc(app.clone()));
        }

        use super::environment::config::SetupPages;
        let start_page = SetupPages::pages()
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

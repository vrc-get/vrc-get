// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

#[cfg_attr(windows, path = "cmd_start_win.rs")]
#[cfg_attr(not(windows), path = "cmd_start_basic.rs")]
mod cmd_start;

mod commands;
mod config;
mod logging;
mod templates;

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
        }))
        .invoke_handler(commands::handlers())
        .setup(move |app| {
            app.manage(commands::new_env_state(io));
            commands::startup(app);
            Ok(())
        })
        .build(tauri_context())
        .expect("error while building tauri application");
    logging::set_app_handle(app.handle());
    app.run(|_, _| {})
}

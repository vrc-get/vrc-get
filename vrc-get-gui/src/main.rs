// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

mod commands;
mod logging;

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
    logging::initialize_logger();

    #[cfg(debug_assertions)]
    commands::export_ts();

    let app = tauri::Builder::default()
        .invoke_handler(commands::handlers())
        .setup(|app| {
            app.manage(commands::new_env_state());
            Ok(())
        })
        .build(tauri_context())
        .expect("error while building tauri application");
    logging::set_app_handle(app.handle());
    app.run(|_, _| {})
}

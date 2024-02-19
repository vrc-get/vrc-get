// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
    tauri::Builder::default()
        .run(tauri_context())
        .expect("error while running tauri application");
}

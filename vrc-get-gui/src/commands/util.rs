use crate::commands::prelude::*;
use crate::logging::LogEntry;

#[tauri::command]
#[specta::specta]
pub async fn util_open(path: String) -> Result<(), RustError> {
    open::that(path).map_err(RustError::unrecoverable)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn util_get_log_entries() -> Vec<LogEntry> {
    crate::logging::get_log_entries()
}

#[tauri::command]
#[specta::specta]
pub fn util_get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

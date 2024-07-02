use std::sync::atomic::{AtomicU32, Ordering};

use tauri::updater::UpdateResponse;
use tauri::{AppHandle, State, Wry};
use tokio::sync::Mutex;

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

pub struct UpdateResponseInfo {
    pub version: u32,
    pub response: UpdateResponse<Wry>,
}

pub struct UpdateResponseHolder {
    changes_info: Option<Box<UpdateResponseInfo>>,
}

impl UpdateResponseHolder {
    pub fn new() -> Self {
        Self { changes_info: None }
    }

    fn update(&mut self, response: UpdateResponse<Wry>) -> u32 {
        static CHANGES_GLOBAL_INDEXER: AtomicU32 = AtomicU32::new(0);
        let version = CHANGES_GLOBAL_INDEXER.fetch_add(1, Ordering::SeqCst);
        self.changes_info = Some(Box::new(UpdateResponseInfo { version, response }));
        version
    }

    fn take(&mut self) -> Option<UpdateResponseInfo> {
        self.changes_info.take().map(|x| *x)
    }
}

pub async fn check_for_update(
    app_handle: AppHandle,
) -> tauri::updater::Result<UpdateResponse<Wry>> {
    tauri::updater::builder(app_handle)
        .skip_events()
        .endpoints(&["https://vrc-get.anatawa12.com/api/gui/tauri-updater.json".into()])
        .check()
        .await
}

#[derive(serde::Serialize, specta::Type)]
pub struct CheckForUpdateResponse {
    version: u32,
    is_update_available: bool,
    current_version: String,
    latest_version: String,
    update_description: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn util_check_for_update(
    app_handle: AppHandle,
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<CheckForUpdateResponse, RustError> {
    let response = check_for_update(app_handle).await?;
    let is_update_available = response.is_update_available();
    let current_version = response.current_version().to_string();
    let latest_version = response.latest_version().to_string();
    let update_description = response.body().map(|s| s.to_string());

    let mut state = state.lock().await;
    let state = &mut *state;
    let version = state.update_response_holder.update(response);
    Ok(CheckForUpdateResponse {
        version,
        is_update_available,
        current_version,
        latest_version,
        update_description,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn util_install_and_upgrade(
    state: State<'_, Mutex<EnvironmentState>>,
    app_handle: AppHandle,
    version: u32,
) -> Result<(), RustError> {
    let Some(response) = ({
        let mut state = state.lock().await;
        let state = &mut *state;
        state.update_response_holder.take()
    }) else {
        return Err(RustError::unrecoverable("No update response found"));
    };

    if response.version != version {
        return Err(RustError::unrecoverable("Update data version mismatch"));
    }

    response.response.download_and_install().await?;

    app_handle.restart();
    unreachable!("app_handle.restart() should restart the app");
}

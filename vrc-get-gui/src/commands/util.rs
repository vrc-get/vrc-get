use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use tauri::updater::UpdateResponse;
use tauri::{AppHandle, State, Wry};
use tokio::fs::create_dir_all;
use tokio::sync::Mutex;
use vrc_get_vpm::io::DefaultEnvironmentIo;

use crate::commands::prelude::*;
use crate::config::GuiConfigState;
use crate::logging::LogEntry;
use crate::utils::find_existing_parent_dir_or_home;

#[derive(serde::Deserialize, specta::Type)]
#[allow(clippy::enum_variant_names)]
pub enum OpenOptions {
    ErrorIfNotExists,
    CreateFolderIfNotExists,
    OpenParentIfNotExists,
}

#[tauri::command]
#[specta::specta]
pub async fn util_open(path: String, if_not_exists: OpenOptions) -> Result<(), RustError> {
    let path = Path::new(&path);
    if !path.exists() {
        match if_not_exists {
            OpenOptions::ErrorIfNotExists => {
                return Err(RustError::unrecoverable("Path does not exist"));
            }
            OpenOptions::CreateFolderIfNotExists => {
                create_dir_all(&path).await?;
                open::that(path)?;
            }
            OpenOptions::OpenParentIfNotExists => {
                open::that(find_existing_parent_dir_or_home(path).as_os_str())?;
            }
        }
    } else {
        open::that(path)?;
    }
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
    stable: bool,
) -> tauri::updater::Result<UpdateResponse<Wry>> {
    let endpoint = if stable {
        "https://vrc-get.anatawa12.com/api/gui/tauri-updater.json"
    } else {
        "https://vrc-get.anatawa12.com/api/gui/tauri-updater-beta.json"
    };
    tauri::updater::builder(app_handle)
        .skip_events()
        .endpoints(&[endpoint.into()])
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
    config: State<'_, GuiConfigState>,
    io: State<'_, DefaultEnvironmentIo>,
) -> Result<CheckForUpdateResponse, RustError> {
    let stable = config.load(&io).await?.release_channel == "stable";
    let response = check_for_update(app_handle, stable).await?;
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

#[cfg(windows)]
#[tauri::command]
#[specta::specta]
pub async fn util_is_bad_hostname() -> Result<bool, RustError> {
    unsafe {
        use windows::Win32::NetworkManagement::IpHelper::{GetNetworkParams, FIXED_INFO_W2KSP1};
        let mut len = 0;
        // ignore error since expecting ERROR_BUFFER_OVERFLOW
        GetNetworkParams(None, &mut len).ok().ok();
        let memory = vec![0u8; len as usize];
        let ptr = memory.as_ptr() as *mut FIXED_INFO_W2KSP1;
        GetNetworkParams(Some(ptr), &mut len)
            .ok()
            .map_err(RustError::unrecoverable)?;
        let info = &*ptr;
        Ok(info
            .HostName
            .iter()
            .take_while(|&&c| c != 0)
            .any(|&c| c < 0))
    }
}

#[cfg(not(windows))]
#[tauri::command]
#[specta::specta]
pub async fn util_is_bad_hostname() -> Result<bool, RustError> {
    Ok(false)
}

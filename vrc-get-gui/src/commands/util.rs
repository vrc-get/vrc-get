use std::path::Path;

use crate::commands::async_command::{AsyncCallResult, With, async_command};
use crate::commands::environment::settings::TauriPickProjectDefaultPathResult;
use crate::commands::prelude::*;
use crate::logging::LogEntry;
use crate::os::open_that;
use crate::utils::find_existing_parent_dir_or_home;
use tauri::{AppHandle, State, Window};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_updater::{Update, UpdaterExt};
use url::Url;

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
                super::create_dir_all_with_err(&path).await?;
                open_that(path)?;
            }
            OpenOptions::OpenParentIfNotExists => {
                open_that(find_existing_parent_dir_or_home(path).as_os_str())?;
            }
        }
    } else {
        open_that(path)?;
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn util_open_url(url: String) -> Result<(), RustError> {
    open_that(url)?;
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

pub async fn check_for_update(
    app_handle: AppHandle,
    stable: bool,
) -> tauri_plugin_updater::Result<Option<Update>> {
    let endpoint = if stable {
        Url::parse("https://vrc-get.anatawa12.com/api/gui/tauri-updater.json").unwrap()
    } else {
        Url::parse("https://vrc-get.anatawa12.com/api/gui/tauri-updater-beta.json").unwrap()
    };
    app_handle
        .updater_builder()
        .endpoints(vec![endpoint])
        .unwrap()
        .build()?
        .check()
        .await
}

#[derive(serde::Serialize, specta::Type)]
pub struct CheckForUpdateResponse {
    version: u32,
    current_version: String,
    latest_version: String,
    update_description: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn util_check_for_update(
    app_handle: AppHandle,
    updater_state: State<'_, UpdaterState>,
    config: State<'_, GuiConfigState>,
) -> Result<Option<CheckForUpdateResponse>, RustError> {
    let stable = config.get().release_channel == "stable";
    let Some(response) = check_for_update(app_handle, stable).await? else {
        return Ok(None);
    };
    let current_version = response.current_version.clone();
    let latest_version = response.version.clone();
    let update_description = response.body.clone();

    let version = updater_state.set(response);
    Ok(Some(CheckForUpdateResponse {
        version,
        current_version,
        latest_version,
        update_description,
    }))
}

#[derive(serde::Serialize, specta::Type, Clone)]
#[serde(tag = "type")]
pub enum InstallUpgradeProgress {
    DownloadProgress { received: usize, total: Option<u64> },
    DownloadComplete,
}

#[tauri::command]
#[specta::specta]
pub async fn util_install_and_upgrade(
    updater_state: State<'_, UpdaterState>,
    app_handle: AppHandle,
    window: Window,
    channel: String,
    version: u32,
) -> Result<AsyncCallResult<InstallUpgradeProgress, ()>, RustError> {
    async_command(channel, window, async move {
        let Some(response) = updater_state.take() else {
            return Err(RustError::unrecoverable("No update response found"));
        };

        if response.version() != version {
            return Err(RustError::unrecoverable("Update data version mismatch"));
        }

        With::<InstallUpgradeProgress>::continue_async(move |ctx| async move {
            response
                .into_data()
                .download_and_install(
                    |received, total| {
                        ctx.emit(InstallUpgradeProgress::DownloadProgress { received, total })
                            .ok();
                    },
                    || {
                        ctx.emit(InstallUpgradeProgress::DownloadComplete).ok();
                    },
                )
                .await?;

            app_handle.restart();
        })
    })
    .await
}

#[cfg(windows)]
#[tauri::command]
#[specta::specta]
pub async fn util_is_bad_hostname() -> Result<bool, RustError> {
    unsafe {
        use windows::Win32::NetworkManagement::IpHelper::{FIXED_INFO_W2KSP1, GetNetworkParams};
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

#[tauri::command]
#[specta::specta]
pub async fn util_pick_directory(
    window: Window,
    current: String,
) -> Result<TauriPickProjectDefaultPathResult, RustError> {
    let Some(dir) = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_directory(find_existing_parent_dir_or_home(current.as_ref()))
        .blocking_pick_folder()
        .map(|x| x.into_path_buf())
        .transpose()?
    else {
        return Ok(TauriPickProjectDefaultPathResult::NoFolderSelected);
    };

    let Ok(dir) = dir.into_os_string().into_string() else {
        return Ok(TauriPickProjectDefaultPathResult::InvalidSelection);
    };

    Ok(TauriPickProjectDefaultPathResult::Successful { new_path: dir })
}

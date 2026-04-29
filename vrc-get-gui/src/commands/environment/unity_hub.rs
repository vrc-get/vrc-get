use crate::state::SettingsState;
use log::debug;
use std::io;
use std::mem::forget;
use tauri::State;
use tokio::spawn;
use tokio::sync::Mutex;
use tokio::sync::oneshot;
use vrc_get_vpm::environment::{VccDatabaseConnection, find_unity_hub};
use vrc_get_vpm::io::DefaultEnvironmentIo;
use vrc_get_vpm::unity_hub;

use crate::commands::prelude::*;
use crate::config::UnityHubAccessMethod;

#[derive(Debug, Clone, Copy)]
enum UpdateUnityResultTiny {
    Success,
    NoUnityHub,
    IoError,
}

struct UpdateUnityState {
    waiters: Vec<oneshot::Sender<UpdateUnityResultTiny>>,
}

impl UpdateUnityState {
    fn new() -> Self {
        Self {
            waiters: Vec::new(),
        }
    }
}

static UPDATE_UNITY_PATH_SHARED_STATE: Mutex<Option<UpdateUnityState>> = Mutex::const_new(None);

pub async fn wait_for_unity_path_update() {
    let mut guard = UPDATE_UNITY_PATH_SHARED_STATE.lock().await;
    if let Some(state) = guard.as_mut() {
        let (sender, receiver) = oneshot::channel();
        state.waiters.push(sender);
        drop(guard);
        receiver.await.ok();
    } else {
        drop(guard);
    }
}

pub async fn is_loading_from_unity_hub_in_progress() -> bool {
    let guard = UPDATE_UNITY_PATH_SHARED_STATE.lock().await;
    guard.is_some()
}

pub async fn update_unity_paths_from_unity_hub(
    settings: &SettingsState,
    config: &GuiConfigState,
    io: &DefaultEnvironmentIo,
) -> io::Result<bool> {
    loop {
        let mut guard = UPDATE_UNITY_PATH_SHARED_STATE.lock().await;
        if let Some(state) = guard.as_mut() {
            debug!("update unity paths requested but already updating, waiting for it to finish");
            let (sender, receiver) = oneshot::channel();
            state.waiters.push(sender);
            drop(guard);
            match receiver.await {
                Ok(UpdateUnityResultTiny::Success) => return Ok(true),
                Ok(UpdateUnityResultTiny::NoUnityHub) => return Ok(false),
                Ok(UpdateUnityResultTiny::IoError) => {
                    return Err(io::Error::other("io error"));
                }
                Err(_) => {
                    debug!("previous update failed with panic or was canceled, retrying");
                    // receiver removed, this should mean runner panics or canceled
                    // retry
                    continue;
                }
            }
        } else {
            // The struct to release lock on panic
            struct PanicGuard;
            impl Drop for PanicGuard {
                fn drop(&mut self) {
                    spawn(async {
                        let mut guard = UPDATE_UNITY_PATH_SHARED_STATE.lock().await;
                        *guard = None;
                    });
                }
            }

            *guard = Some(UpdateUnityState::new());

            drop(guard);
            let _panic_guard = PanicGuard;

            debug!("updating unity paths from unity hub");

            let result = update_unity_paths_from_unity_hub_impl(settings, config, io).await;

            debug!("updating unity paths from unity hub finished, notifying waiters");

            let tiny = match result {
                Ok(true) => UpdateUnityResultTiny::Success,
                Ok(false) => UpdateUnityResultTiny::NoUnityHub,
                Err(_) => UpdateUnityResultTiny::IoError,
            };

            let mut guard = UPDATE_UNITY_PATH_SHARED_STATE.lock().await;
            let state = guard.take();
            forget(_panic_guard); // we now have none on the guard
            drop(guard);

            let state = state.unwrap();
            for x in state.waiters {
                x.send(tiny).ok();
            }
            return result;
        }
    }
}

async fn update_unity_paths_from_unity_hub_impl(
    settings: &SettingsState,
    config: &GuiConfigState,
    io: &DefaultEnvironmentIo,
) -> io::Result<bool> {
    let paths_from_hub = match config.get().unity_hub_access_method {
        UnityHubAccessMethod::ReadConfig => unity_hub::load_unity_by_loading_unity_hub_files()
            .await?
            .into_iter()
            .map(|x| (x.version, x.path))
            .collect::<Vec<_>>(),
        UnityHubAccessMethod::CallHub => {
            let unity_hub_path = {
                let mut settings = settings.load_mut(io).await?;
                let Some(unity_hub_path) = find_unity_hub(&mut settings, io).await? else {
                    settings.save().await?;
                    return Ok(false);
                };
                settings.save().await?;
                unity_hub_path
            };

            unity_hub::load_unity_by_calling_unity_hub(unity_hub_path.as_ref()).await?
        }
    };

    {
        let mut connection = VccDatabaseConnection::connect(io).await?;

        connection
            .update_unity_from_unity_hub_and_fs(&paths_from_hub, io)
            .await?;

        connection.save(io).await?;
    }

    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub async fn environment_update_unity_paths_from_unity_hub(
    settings: State<'_, SettingsState>,
    config: State<'_, GuiConfigState>,
    io: State<'_, DefaultEnvironmentIo>,
) -> Result<bool, RustError> {
    Ok(update_unity_paths_from_unity_hub(&settings, &config, &io).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn environment_is_loading_from_unity_hub_in_progress() -> bool {
    is_loading_from_unity_hub_in_progress().await
}

#[tauri::command]
#[specta::specta]
pub async fn environment_wait_for_unity_hub_update() {
    wait_for_unity_path_update().await
}

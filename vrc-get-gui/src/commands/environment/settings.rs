use std::ffi::OsStr;
use std::io;
use std::path::Path;

use log::info;
use serde::Serialize;
use tauri::api::dialog::blocking::FileDialogBuilder;
use tauri::async_runtime::spawn;
use tauri::{AppHandle, State};
use tokio::sync::Mutex;

use crate::commands::prelude::*;
use crate::commands::DEFAULT_UNITY_ARGUMENTS;
use crate::config::GuiConfigState;
use crate::utils::{default_project_path, find_existing_parent_dir_or_home, project_backup_path};
use vrc_get_vpm::io::DefaultEnvironmentIo;
use vrc_get_vpm::{VRCHAT_RECOMMENDED_2022_UNITY, VRCHAT_RECOMMENDED_2022_UNITY_HUB_LINK};

#[derive(Serialize, specta::Type)]
pub struct TauriUnityVersions {
    unity_paths: Vec<(String, String, bool)>,
    recommended_version: String,
    install_recommended_version_link: String,
}

#[tauri::command]
#[specta::specta]
pub async fn environment_unity_versions(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<TauriUnityVersions, RustError> {
    with_environment!(&state, |environment| {
        environment.find_unity_hub().await.ok();

        let unity_paths = environment
            .get_unity_installations()?
            .iter()
            .filter_map(|unity| {
                Some((
                    unity.path().to_string(),
                    unity.version()?.to_string(),
                    unity.loaded_from_hub(),
                ))
            })
            .collect();

        environment.disconnect_litedb();

        Ok(TauriUnityVersions {
            unity_paths,
            recommended_version: VRCHAT_RECOMMENDED_2022_UNITY.to_string(),
            install_recommended_version_link: VRCHAT_RECOMMENDED_2022_UNITY_HUB_LINK.to_string(),
        })
    })
}

#[derive(Serialize, specta::Type)]
pub struct TauriEnvironmentSettings {
    default_project_path: String,
    project_backup_path: String,
    unity_hub: String,
    unity_paths: Vec<(String, String, bool)>,
    show_prerelease_packages: bool,
    backup_format: String,
    release_channel: String,
    use_alcom_for_vcc_protocol: bool,
    default_unity_arguments: Option<Vec<String>>,
}

#[tauri::command]
#[specta::specta]
pub async fn environment_get_settings(
    state: State<'_, Mutex<EnvironmentState>>,
    config: State<'_, GuiConfigState>,
    io: State<'_, DefaultEnvironmentIo>,
) -> Result<TauriEnvironmentSettings, RustError> {
    let config = config.load(&io).await?;
    let backup_format = config.backup_format.to_string();
    let release_channel = config.release_channel.to_string();
    let use_alcom_for_vcc_protocol = config.use_alcom_for_vcc_protocol;
    let default_unity_arguments = config.default_unity_arguments.clone();
    drop(config);

    with_environment!(&state, |environment| {
        environment.find_unity_hub().await.ok();

        let settings = TauriEnvironmentSettings {
            default_project_path: default_project_path(environment).await?.to_string(),
            project_backup_path: project_backup_path(environment).await?.to_string(),
            unity_hub: environment.unity_hub_path().to_string(),
            unity_paths: environment
                .get_unity_installations()?
                .iter()
                .filter_map(|unity| {
                    Some((
                        unity.path().to_string(),
                        unity.version()?.to_string(),
                        unity.loaded_from_hub(),
                    ))
                })
                .collect(),
            show_prerelease_packages: environment.show_prerelease_packages(),
            backup_format,
            release_channel,
            use_alcom_for_vcc_protocol,
            default_unity_arguments,
        };
        environment.disconnect_litedb();
        Ok(settings)
    })
}

#[derive(Serialize, specta::Type)]
#[serde(tag = "type")]
pub enum TauriPickUnityHubResult {
    NoFolderSelected,
    InvalidSelection,
    Successful,
}

#[tauri::command]
#[specta::specta]
pub async fn environment_pick_unity_hub(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<TauriPickUnityHubResult, RustError> {
    let Some(mut path) = with_environment!(&state, |environment| {
        let mut unity_hub = Path::new(environment.unity_hub_path());

        if cfg!(target_os = "macos") {
            // for macos, select .app file instead of the executable binary inside it
            if unity_hub.ends_with("Contents/MacOS/Unity Hub") {
                unity_hub = unity_hub
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap();
            }
        }

        let mut builder = FileDialogBuilder::new();

        if unity_hub.parent().is_some() {
            builder = builder
                .set_directory(unity_hub.parent().unwrap())
                .set_file_name(&unity_hub.file_name().unwrap().to_string_lossy());
        }

        if cfg!(target_os = "macos") {
            builder = builder.add_filter("Application", &["app"]);
        } else if cfg!(target_os = "windows") {
            builder = builder.add_filter("Executable", &["exe"]);
        } else if cfg!(target_os = "linux") {
            // no extension for executable on linux
        }

        builder.pick_file()
    }) else {
        return Ok(TauriPickUnityHubResult::NoFolderSelected);
    };

    // validate / update the file
    #[allow(clippy::if_same_then_else)]
    if cfg!(target_os = "macos") {
        if path.extension().map(|x| x.to_ascii_lowercase()).as_deref() == Some(OsStr::new("app")) {
            // it's app bundle so select the executable inside it
            path.push("Contents/MacOS/Unity Hub");
            if !path.exists() {
                return Ok(TauriPickUnityHubResult::InvalidSelection);
            }
        }
    } else if cfg!(target_os = "windows") {
        // no validation
    } else if cfg!(target_os = "linux") {
        // no validation
    }

    let Ok(path) = path.into_os_string().into_string() else {
        return Ok(TauriPickUnityHubResult::InvalidSelection);
    };

    with_environment!(&state, |environment| {
        environment.set_unity_hub_path(&path);
        environment.save().await?;
    });

    Ok(TauriPickUnityHubResult::Successful)
}

#[derive(Serialize, specta::Type)]
pub enum TauriPickUnityResult {
    NoFolderSelected,
    InvalidSelection,
    AlreadyAdded,
    Successful,
}

#[tauri::command]
#[specta::specta]
pub async fn environment_pick_unity(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<TauriPickUnityResult, RustError> {
    let Some(mut path) = ({
        let mut builder = FileDialogBuilder::new();
        if cfg!(target_os = "macos") {
            builder = builder.add_filter("Application", &["app"]);
        } else if cfg!(target_os = "windows") {
            builder = builder.add_filter("Executable", &["exe"]);
        } else if cfg!(target_os = "linux") {
            // no extension for executable on linux
        }

        builder.pick_file()
    }) else {
        return Ok(TauriPickUnityResult::NoFolderSelected);
    };

    // validate / update the file
    #[allow(clippy::if_same_then_else)]
    if cfg!(target_os = "macos") {
        if path.extension().map(|x| x.to_ascii_lowercase()).as_deref() == Some(OsStr::new("app")) {
            // it's app bundle so select the executable inside it
            path.push("Contents/MacOS/Unity");
            if !path.exists() {
                return Ok(TauriPickUnityResult::InvalidSelection);
            }
        }
    } else if cfg!(target_os = "windows") {
        // no validation
    } else if cfg!(target_os = "linux") {
        // no validation
    }

    let Ok(path) = path.into_os_string().into_string() else {
        return Ok(TauriPickUnityResult::InvalidSelection);
    };

    let unity_version = vrc_get_vpm::unity::call_unity_for_version(path.as_ref()).await?;

    with_environment!(&state, |environment| {
        for x in environment.get_unity_installations()? {
            if x.path() == path {
                return Ok(TauriPickUnityResult::AlreadyAdded);
            }
        }

        match environment
            .add_unity_installation(&path, unity_version)
            .await
        {
            Err(ref e) if e.kind() == io::ErrorKind::InvalidInput => {
                return Ok(TauriPickUnityResult::InvalidSelection)
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {}
        }
        environment.save().await?;
    });

    Ok(TauriPickUnityResult::Successful)
}

#[derive(Serialize, specta::Type)]
#[serde(tag = "type")]
pub enum TauriPickProjectDefaultPathResult {
    NoFolderSelected,
    InvalidSelection,
    Successful { new_path: String },
}

#[tauri::command]
#[specta::specta]
pub async fn environment_pick_project_default_path(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<TauriPickProjectDefaultPathResult, RustError> {
    let Some(dir) = with_environment!(state, |environment| {
        FileDialogBuilder::new()
            .set_directory(find_existing_parent_dir_or_home(
                default_project_path(environment).await?.as_ref(),
            ))
            .pick_folder()
    }) else {
        return Ok(TauriPickProjectDefaultPathResult::NoFolderSelected);
    };

    let Ok(dir) = dir.into_os_string().into_string() else {
        return Ok(TauriPickProjectDefaultPathResult::InvalidSelection);
    };

    with_environment!(&state, |environment| {
        environment.set_default_project_path(&dir);
        environment.save().await?;
    });

    Ok(TauriPickProjectDefaultPathResult::Successful { new_path: dir })
}

#[derive(Serialize, specta::Type)]
#[serde(tag = "type")]
pub enum TauriPickProjectBackupPathResult {
    NoFolderSelected,
    InvalidSelection,
    Successful,
}

#[tauri::command]
#[specta::specta]
pub async fn environment_pick_project_backup_path(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<TauriPickProjectBackupPathResult, RustError> {
    let Some(dir) = with_environment!(state, |environment| {
        FileDialogBuilder::new()
            .set_directory(find_existing_parent_dir_or_home(
                project_backup_path(environment).await?.as_ref(),
            ))
            .pick_folder()
    }) else {
        return Ok(TauriPickProjectBackupPathResult::NoFolderSelected);
    };

    let Ok(dir) = dir.into_os_string().into_string() else {
        return Ok(TauriPickProjectBackupPathResult::InvalidSelection);
    };

    with_environment!(&state, |environment| {
        environment.set_project_backup_path(&dir);
        environment.save().await?;
    });

    Ok(TauriPickProjectBackupPathResult::Successful)
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_show_prerelease_packages(
    state: State<'_, Mutex<EnvironmentState>>,
    value: bool,
) -> Result<(), RustError> {
    with_environment!(&state, |environment| {
        environment.set_show_prerelease_packages(value);
        environment.save().await?;
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_backup_format(
    config: State<'_, GuiConfigState>,
    io: State<'_, DefaultEnvironmentIo>,
    backup_format: String,
) -> Result<(), RustError> {
    let mut config = config.load_mut(&io).await?;
    config.backup_format = backup_format;
    config.save().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_release_channel(
    config: State<'_, GuiConfigState>,
    io: State<'_, DefaultEnvironmentIo>,
    release_channel: String,
) -> Result<(), RustError> {
    let mut config = config.load_mut(&io).await?;
    config.release_channel = release_channel;
    config.save().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_use_alcom_for_vcc_protocol(
    app: AppHandle,
    config: State<'_, GuiConfigState>,
    io: State<'_, DefaultEnvironmentIo>,
    use_alcom_for_vcc_protocol: bool,
) -> Result<(), RustError> {
    let mut config = config.load_mut(&io).await?;
    info!("setting use_alcom_for_vcc_protocol to {use_alcom_for_vcc_protocol}");
    config.use_alcom_for_vcc_protocol = use_alcom_for_vcc_protocol;
    config.save().await?;
    if use_alcom_for_vcc_protocol {
        spawn(crate::deep_link_support::deep_link_install_vcc(app));
    } else {
        spawn(crate::deep_link_support::deep_link_uninstall_vcc(app));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_get_default_unity_arguments(
    config: State<'_, GuiConfigState>,
    io: State<'_, DefaultEnvironmentIo>,
) -> Result<Vec<String>, RustError> {
    Ok(config
        .load(&io)
        .await?
        .default_unity_arguments
        .clone()
        .unwrap_or_else(|| {
            DEFAULT_UNITY_ARGUMENTS
                .iter()
                .copied()
                .map(String::from)
                .collect()
        }))
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_default_unity_arguments(
    config: State<'_, GuiConfigState>,
    io: State<'_, DefaultEnvironmentIo>,
    default_unity_arguments: Option<Vec<String>>,
) -> Result<(), RustError> {
    let mut config = config.load_mut(&io).await?;
    config.default_unity_arguments = default_unity_arguments;
    config.save().await?;
    Ok(())
}

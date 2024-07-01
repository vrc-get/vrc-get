use std::ffi::OsStr;
use std::io;
use std::path::Path;

use log::info;
use serde::Serialize;
use tauri::api::dialog::blocking::FileDialogBuilder;
use tauri::State;
use tokio::sync::Mutex;

use vrc_get_vpm::io::EnvironmentIo;
use vrc_get_vpm::{
    EnvironmentIoHolder, VRCHAT_RECOMMENDED_2022_UNITY, VRCHAT_RECOMMENDED_2022_UNITY_HUB_LINK,
};

use crate::commands::prelude::*;

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
}

#[tauri::command]
#[specta::specta]
pub async fn environment_get_settings(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<TauriEnvironmentSettings, RustError> {
    with_environment!(&state, |environment, config| {
        environment.find_unity_hub().await.ok();

        let settings = TauriEnvironmentSettings {
            default_project_path: environment.default_project_path().to_string(),
            project_backup_path: environment.project_backup_path().to_string(),
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
            backup_format: config.backup_format.to_string(),
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
        // default path may not be exists so create here
        // Note: keep in sync with vrc-get-vpm/src/environment/settings.rs
        let mut default_path = environment.io().resolve("".as_ref());
        default_path.pop();
        default_path.push("VRChatProjects");
        println!("default_path: {:?}", default_path.display());
        if default_path.as_path() == Path::new(environment.default_project_path()) {
            tokio::fs::create_dir_all(&default_path).await.ok();
        }

        FileDialogBuilder::new()
            .set_directory(environment.default_project_path())
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
        // backup folder may not be exists so create here
        // Note: keep in sync with vrc-get-vpm/src/environment/settings.rs
        let default_path = environment.io().resolve("Project Backups".as_ref());
        if default_path.as_path() == Path::new(environment.project_backup_path()) {
            tokio::fs::create_dir_all(&default_path).await.ok();
        }

        FileDialogBuilder::new()
            .set_directory(environment.project_backup_path())
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
    state: State<'_, Mutex<EnvironmentState>>,
    backup_format: String,
) -> Result<(), RustError> {
    with_config!(&state, |mut config| {
        info!("setting backup_format to {backup_format}");
        config.backup_format = backup_format;
        config.save().await?;
        Ok(())
    })
}

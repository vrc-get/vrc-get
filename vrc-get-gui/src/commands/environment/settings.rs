use std::ffi::OsStr;
use std::io;
use std::path::Path;

use crate::commands::DEFAULT_UNITY_ARGUMENTS;
use crate::commands::prelude::*;
use crate::config::UnityHubAccessMethod;
use crate::utils::{default_project_path, find_existing_parent_dir_or_home, project_backup_path};
use log::info;
use serde::Serialize;
use tauri::async_runtime::spawn;
use tauri::{AppHandle, State, Window};
use tauri_plugin_dialog::DialogExt;
use vrc_get_vpm::environment::{VccDatabaseConnection, find_unity_hub};
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
    io: State<'_, DefaultEnvironmentIo>,
) -> Result<TauriUnityVersions, RustError> {
    let connection = VccDatabaseConnection::connect(io.inner()).await?;

    let unity_paths = connection
        .get_unity_installations()
        .iter()
        .filter_map(|unity| {
            Some((
                unity.path()?.to_string(),
                unity.version()?.to_string(),
                unity.loaded_from_hub(),
            ))
        })
        .collect();

    Ok(TauriUnityVersions {
        unity_paths,
        recommended_version: VRCHAT_RECOMMENDED_2022_UNITY.to_string(),
        install_recommended_version_link: VRCHAT_RECOMMENDED_2022_UNITY_HUB_LINK.to_string(),
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
    gui_animation: bool,
    unity_hub_access_method: UnityHubAccessMethod,
    exclude_vpm_packages_from_backup: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn environment_get_settings(
    settings: State<'_, SettingsState>,
    config: State<'_, GuiConfigState>,
    io: State<'_, DefaultEnvironmentIo>,
) -> Result<TauriEnvironmentSettings, RustError> {
    let backup_format;
    let release_channel;
    let use_alcom_for_vcc_protocol;
    let default_unity_arguments;
    let unity_paths;
    let unity_hub;
    let default_project_path;
    let project_backup_path;
    let show_prerelease_packages;
    let gui_animation;
    let unity_hub_access_method;
    let exclude_vpm_packages_from_backup;

    {
        let config = config.get();
        backup_format = config.backup_format.to_string();
        release_channel = config.release_channel.to_string();
        use_alcom_for_vcc_protocol = config.use_alcom_for_vcc_protocol;
        default_unity_arguments = config.default_unity_arguments.clone();
        gui_animation = config.gui_animation;
        unity_hub_access_method = config.unity_hub_access_method;
        exclude_vpm_packages_from_backup = config.exclude_vpm_packages_from_backup;
    }

    {
        let connection = VccDatabaseConnection::connect(io.inner()).await?;

        unity_paths = connection
            .get_unity_installations()
            .into_iter()
            .filter_map(|unity| {
                Some((
                    unity.path()?.to_string(),
                    unity.version()?.to_string(),
                    unity.loaded_from_hub(),
                ))
            })
            .collect();
    }

    {
        let mut settings = settings.load_mut(io.inner()).await?;

        find_unity_hub(&mut settings, io.inner()).await?;
        unity_hub = settings.unity_hub_path().to_string();
        default_project_path = crate::utils::default_project_path(&mut settings).to_string();
        project_backup_path = crate::utils::project_backup_path(&mut settings).to_string();
        show_prerelease_packages = settings.show_prerelease_packages();

        settings.save().await?;
    }

    Ok(TauriEnvironmentSettings {
        default_project_path,
        project_backup_path,
        unity_hub,
        unity_paths,
        show_prerelease_packages,
        backup_format,
        release_channel,
        use_alcom_for_vcc_protocol,
        default_unity_arguments,
        gui_animation,
        unity_hub_access_method,
        exclude_vpm_packages_from_backup,
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
    settings: State<'_, SettingsState>,
    io: State<'_, DefaultEnvironmentIo>,
    window: Window,
) -> Result<TauriPickUnityHubResult, RustError> {
    let Some(mut path) = ({
        let settings = settings.load(io.inner()).await?;
        let mut unity_hub = Path::new(settings.unity_hub_path());

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

        let mut builder = window.dialog().file().set_parent(&window);

        if unity_hub.parent().is_some() {
            builder = builder
                .set_directory(unity_hub.parent().unwrap())
                .set_file_name(unity_hub.file_name().unwrap().to_string_lossy());
        }

        if cfg!(target_os = "macos") {
            builder = builder.add_filter("Application", &["app"]);
        } else if cfg!(target_os = "windows") {
            builder = builder.add_filter("Executable", &["exe"]);
        } else if cfg!(target_os = "linux") {
            // no extension for executable on linux
        }

        builder
            .blocking_pick_file()
            .map(|x| x.into_path_buf())
            .transpose()?
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

    let mut settings = settings.load_mut(io.inner()).await?;
    settings.set_unity_hub_path(&path);
    settings.save().await?;

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
    io: State<'_, DefaultEnvironmentIo>,
    window: Window,
) -> Result<TauriPickUnityResult, RustError> {
    let Some(mut path) = ({
        let mut builder = window.dialog().file().set_parent(&window);
        if cfg!(target_os = "macos") {
            builder = builder.add_filter("Application", &["app"]);
        } else if cfg!(target_os = "windows") {
            builder = builder.add_filter("Executable", &["exe"]);
        } else if cfg!(target_os = "linux") {
            // no extension for executable on linux
        }

        builder
            .blocking_pick_file()
            .map(|x| x.into_path_buf())
            .transpose()?
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

    {
        let mut connection = VccDatabaseConnection::connect(io.inner()).await?;

        for x in connection.get_unity_installations() {
            if x.path() == Some(&path) {
                return Ok(TauriPickUnityResult::AlreadyAdded);
            }
        }

        match connection.add_unity_installation(&path, unity_version) {
            Err(ref e) if e.kind() == io::ErrorKind::InvalidInput => {
                return Ok(TauriPickUnityResult::InvalidSelection);
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {}
        }

        connection.save(io.inner()).await?;
    }

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
    settings: State<'_, SettingsState>,
    io: State<'_, DefaultEnvironmentIo>,
    window: Window,
) -> Result<TauriPickProjectDefaultPathResult, RustError> {
    let mut settings = settings.load_mut(io.inner()).await?;
    let default_project_path = default_project_path(&mut settings);
    let Some(dir) = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_directory(find_existing_parent_dir_or_home(
            default_project_path.as_ref(),
        ))
        .blocking_pick_folder()
        .map(|x| x.into_path_buf())
        .transpose()?
    else {
        settings.maybe_save().await?;
        return Ok(TauriPickProjectDefaultPathResult::NoFolderSelected);
    };

    let Ok(dir) = dir.into_os_string().into_string() else {
        settings.maybe_save().await?;
        return Ok(TauriPickProjectDefaultPathResult::InvalidSelection);
    };

    settings.set_default_project_path(&dir);
    settings.save().await?;

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
    settings: State<'_, SettingsState>,
    io: State<'_, DefaultEnvironmentIo>,
    window: Window,
) -> Result<TauriPickProjectBackupPathResult, RustError> {
    let mut settings = settings.load_mut(io.inner()).await?;
    let project_backup_path = project_backup_path(&mut settings);
    let Some(dir) = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_directory(find_existing_parent_dir_or_home(
            project_backup_path.as_ref(),
        ))
        .blocking_pick_folder()
        .map(|x| x.into_path_buf())
        .transpose()?
    else {
        return Ok(TauriPickProjectBackupPathResult::NoFolderSelected);
    };

    let Ok(dir) = dir.into_os_string().into_string() else {
        return Ok(TauriPickProjectBackupPathResult::InvalidSelection);
    };

    settings.set_project_backup_path(&dir);
    settings.save().await?;

    Ok(TauriPickProjectBackupPathResult::Successful)
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_show_prerelease_packages(
    io: State<'_, DefaultEnvironmentIo>,
    settings: State<'_, SettingsState>,
    value: bool,
) -> Result<(), RustError> {
    let mut settings = settings.load_mut(io.inner()).await?;
    settings.set_show_prerelease_packages(value);
    settings.save().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_backup_format(
    config: State<'_, GuiConfigState>,
    backup_format: String,
) -> Result<(), RustError> {
    let mut config = config.load_mut().await?;
    config.backup_format = backup_format;
    config.save().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_exclude_vpm_packages_from_backup(
    config: State<'_, GuiConfigState>,
    exclude_vpm_packages_from_backup: bool,
) -> Result<(), RustError> {
    let mut config = config.load_mut().await?;
    config.exclude_vpm_packages_from_backup = exclude_vpm_packages_from_backup;
    config.save().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_release_channel(
    config: State<'_, GuiConfigState>,
    release_channel: String,
) -> Result<(), RustError> {
    let mut config = config.load_mut().await?;
    config.release_channel = release_channel;
    config.save().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_use_alcom_for_vcc_protocol(
    app: AppHandle,
    config: State<'_, GuiConfigState>,
    use_alcom_for_vcc_protocol: bool,
) -> Result<(), RustError> {
    let mut config = config.load_mut().await?;
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
) -> Result<Vec<String>, RustError> {
    Ok(config
        .get()
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
    default_unity_arguments: Option<Vec<String>>,
) -> Result<(), RustError> {
    let mut config = config.load_mut().await?;
    config.default_unity_arguments = default_unity_arguments;
    config.save().await?;
    Ok(())
}

use std::ffi::OsStr;
use std::fmt::Display;
use std::io;
use std::num::Wrapping;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::process::Stdio;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU32, Ordering};

use futures::prelude::*;
use log::{error, info, warn};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use specta::specta;
use tauri::api::dialog::blocking::FileDialogBuilder;
use tauri::async_runtime::Mutex;
use tauri::{
    generate_handler, App, AppHandle, Invoke, LogicalSize, Manager, State, Window, WindowEvent,
};
use tokio::fs::read_dir;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use crate::commands::async_command::immediate;
use async_command::{async_command, AsyncCallResult, AsyncCommandContext, With};
use vrc_get_vpm::environment::UserProject;
use vrc_get_vpm::io::{DefaultEnvironmentIo, DefaultProjectIo, DirEntry, EnvironmentIo, IoTrait};
use vrc_get_vpm::repository::RemoteRepository;
use vrc_get_vpm::unity_project::pending_project_changes::{
    ConflictInfo, PackageChange, RemoveReason,
};
use vrc_get_vpm::unity_project::{AddPackageOperation, PendingProjectChanges};
use vrc_get_vpm::version::Version;
use vrc_get_vpm::{
    unity_hub, EnvironmentIoHolder, PackageCollection, PackageInfo, PackageManifest, ProjectType,
    VersionSelector, VRCHAT_RECOMMENDED_2022_UNITY, VRCHAT_RECOMMENDED_2022_UNITY_HUB_LINK,
};

use crate::config::GuiConfigHolder;
use crate::logging::LogEntry;
use crate::specta::IndexMapV2;

mod async_command;

pub(crate) fn handlers() -> impl Fn(Invoke) + Send + Sync + 'static {
    generate_handler![
        environment_language,
        environment_set_language,
        environment_theme,
        environment_set_theme,
        environment_projects,
        environment_add_project_with_picker,
        environment_remove_project,
        environment_remove_project_by_path,
        environment_copy_project_for_migration,
        environment_set_favorite_project,
        environment_get_project_sorting,
        environment_set_project_sorting,
        environment_refetch_packages,
        environment_packages,
        environment_repositories_info,
        environment_hide_repository,
        environment_show_repository,
        environment_set_hide_local_user_packages,
        environment_unity_versions,
        environment_get_settings,
        environment_pick_unity_hub,
        environment_pick_unity,
        environment_pick_project_default_path,
        environment_pick_project_backup_path,
        environment_set_show_prerelease_packages,
        environment_set_backup_format,
        environment_download_repository,
        environment_add_repository,
        environment_remove_repository,
        environment_project_creation_information,
        environment_check_project_name,
        environment_create_project,
        project_details,
        project_install_package,
        project_install_multiple_package,
        project_upgrade_multiple_package,
        project_resolve,
        project_remove_packages,
        project_apply_pending_changes,
        project_migrate_project_to_2022,
        project_call_unity_for_migration,
        project_migrate_project_to_vpm,
        project_open_unity,
        project_is_unity_launching,
        project_create_backup,
        util_open,
        util_get_log_entries,
        util_get_version,
        crate::deep_link_support::deep_link_has_add_repository,
        crate::deep_link_support::deep_link_take_add_repository,
        crate::deep_link_support::deep_link_install_vcc,
    ]
}

#[cfg(debug_assertions)]
pub(crate) fn export_ts() {
    tauri_specta::ts::export_with_cfg(
        specta::collect_types![
            environment_language,
            environment_set_language,
            environment_theme,
            environment_set_theme,
            environment_projects,
            environment_add_project_with_picker,
            environment_remove_project,
            environment_remove_project_by_path,
            environment_copy_project_for_migration,
            environment_set_favorite_project,
            environment_get_project_sorting,
            environment_set_project_sorting,
            environment_refetch_packages,
            environment_packages,
            environment_repositories_info,
            environment_hide_repository,
            environment_show_repository,
            environment_set_hide_local_user_packages,
            environment_unity_versions,
            environment_get_settings,
            environment_pick_unity_hub,
            environment_pick_unity,
            environment_pick_project_default_path,
            environment_pick_project_backup_path,
            environment_set_show_prerelease_packages,
            environment_set_backup_format,
            environment_download_repository,
            environment_add_repository,
            environment_remove_repository,
            environment_project_creation_information,
            environment_check_project_name,
            environment_create_project,
            project_details,
            project_install_package,
            project_install_multiple_package,
            project_upgrade_multiple_package,
            project_resolve,
            project_remove_packages,
            project_apply_pending_changes,
            project_migrate_project_to_2022,
            project_call_unity_for_migration,
            project_migrate_project_to_vpm,
            project_open_unity,
            project_is_unity_launching,
            project_create_backup,
            util_open,
            util_get_log_entries,
            util_get_version,
            crate::deep_link_support::deep_link_has_add_repository,
            crate::deep_link_support::deep_link_take_add_repository,
            crate::deep_link_support::deep_link_install_vcc,
        ]
        .unwrap(),
        specta::ts::ExportConfiguration::new().bigint(specta::ts::BigIntExportBehavior::Number),
        "lib/bindings.ts",
    )
    .unwrap();
}

pub(crate) fn new_env_state(io: DefaultEnvironmentIo) -> impl Send + Sync + 'static {
    Mutex::new(EnvironmentState::new(io))
}

macro_rules! with_environment {
    ($state: expr, |$environment: pat_param$(, $config: pat_param)?| $body: expr) => {{
        let mut state = $state.lock().await;
        let state = &mut *state;
        let $environment = state
            .environment
            .get_environment_mut(UpdateRepositoryMode::None, &state.io)
            .await?;
        $(let $config = state.config.load(&state.io).await?;)?
        $body
    }};
}

macro_rules! with_config {
    ($state: expr, |$config: pat_param| $body: expr) => {{
        let mut state = $state.lock().await;
        let state = &mut *state;
        let $config = state.config.load(&state.io).await?;
        $body
    }};
}

pub(crate) fn startup(app: &mut App) {
    let handle = app.handle();
    tauri::async_runtime::spawn(async move {
        let state = handle.state();
        if let Err(e) = update_unity_hub(state).await {
            error!("failed to update unity from unity hub: {e}");
        }
    });

    let handle = app.handle();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = open_main(handle).await {
            error!("failed to open main window: {e}");
        }
    });

    async fn update_unity_hub(state: State<'_, Mutex<EnvironmentState>>) -> Result<(), io::Error> {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let unity_hub_path = with_environment!(&state, |environment| {
            let Some(unity_hub_path) = environment.find_unity_hub().await? else {
                error!("Unity Hub not found");
                return Ok(());
            };
            environment.save().await?;
            unity_hub_path
        });

        let paths_from_hub = unity_hub::get_unity_from_unity_hub(unity_hub_path.as_ref()).await?;

        with_environment!(&state, |environment| {
            environment
                .update_unity_from_unity_hub_and_fs(&paths_from_hub)
                .await?;

            environment.save().await?;
        });

        info!("finished updating unity from unity hub");
        Ok(())
    }

    async fn open_main(app: AppHandle) -> tauri::Result<()> {
        let state: State<'_, Mutex<EnvironmentState>> = app.state();
        let config = with_config!(state, |config| config.clone());

        let query = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("lang", &config.language)
            .append_pair("theme", &config.theme)
            .finish();

        let window = tauri::WindowBuilder::new(
            &app,
            "main", /* the unique window label */
            tauri::WindowUrl::App(format!("/projects/?{query}").into()),
        )
        .title("ALCOM")
        .resizable(true)
        .on_navigation(|url| {
            if cfg!(debug_assertions) {
                url.host_str() == Some("localhost")
            } else if cfg!(windows) {
                url.scheme() == "https" && url.host_str() == Some("tauri.localhost")
            } else {
                url.scheme() == "tauri"
            }
        })
        .build()?;

        // keep original size if it's too small
        if config.window_size.width > 100 && config.window_size.height > 100 {
            window.set_size(LogicalSize {
                width: config.window_size.width,
                height: config.window_size.height,
            })?;
        }

        window.set_fullscreen(config.fullscreen)?;

        let cloned = window.clone();

        let resize_debounce: std::sync::Mutex<Option<tauri::async_runtime::JoinHandle<()>>> =
            std::sync::Mutex::new(None);

        #[allow(clippy::single_match)]
        window.on_window_event(move |e| match e {
            WindowEvent::Resized(size) => {
                let logical = size
                    .to_logical::<u32>(cloned.current_monitor().unwrap().unwrap().scale_factor());

                if logical.width < 100 || logical.height < 100 {
                    // ignore too small sizes
                    // this is generally caused by the window being minimized
                    return;
                }

                let fullscreen = cloned.is_fullscreen().unwrap();

                let mut resize_debounce = resize_debounce.lock().unwrap();

                if let Some(resize_debounce) = resize_debounce.as_ref() {
                    resize_debounce.abort();
                }

                let cloned = cloned.clone();

                *resize_debounce = Some(tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                    if let Err(e) = save_window_size(cloned.state(), logical, fullscreen).await {
                        error!("failed to save window size: {e}");
                    }
                }));
            }
            _ => {}
        });

        async fn save_window_size(
            state: State<'_, Mutex<EnvironmentState>>,
            size: LogicalSize<u32>,
            fullscreen: bool,
        ) -> tauri::Result<()> {
            info!(
                "saving window size: {}x{}, full: {}",
                size.width, size.height, fullscreen
            );
            with_config!(state, |mut config| {
                if fullscreen {
                    config.fullscreen = true;
                } else {
                    config.fullscreen = false;
                    config.window_size.width = size.width;
                    config.window_size.height = size.height;
                }
                config.save().await?;
            });
            Ok(())
        }

        Ok(())
    }
}

type Environment = vrc_get_vpm::Environment<reqwest::Client, DefaultEnvironmentIo>;
type UnityProject = vrc_get_vpm::UnityProject<DefaultProjectIo>;

async fn new_environment(io: &DefaultEnvironmentIo) -> io::Result<Environment> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("vrc-get-litedb/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("building client");
    Environment::load(Some(client), io.clone()).await
}

async fn update_project_last_modified(env: &mut Environment, project_dir: &Path) {
    async fn inner(env: &mut Environment, project_dir: &Path) -> Result<(), io::Error> {
        env.update_project_last_modified(project_dir)?;
        env.save().await?;
        Ok(())
    }

    if let Err(err) = inner(env, project_dir).await {
        eprintln!("error updating project updated_at on vcc: {err}");
    }
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[specta(export)]
#[serde(tag = "type")]
enum RustError {
    Unrecoverable { message: String },
}

impl RustError {
    fn unrecoverable<T: Display>(value: T) -> Self {
        error!("{value}");
        Self::Unrecoverable {
            message: value.to_string(),
        }
    }
}

impl<E: Display> From<E> for RustError {
    fn from(value: E) -> Self {
        RustError::unrecoverable(format!("io error: {value}"))
    }
}

unsafe impl Send for EnvironmentState {}

unsafe impl Sync for EnvironmentState {}

struct EnvironmentState {
    io: DefaultEnvironmentIo,
    environment: EnvironmentHolder,
    config: GuiConfigHolder,
    packages: Option<NonNull<[PackageInfo<'static>]>>,
    // null or reference to
    projects: Box<[UserProject]>,
    projects_version: Wrapping<u32>,
    changes_info: ChangesInfoHolder,
}

struct PendingProjectChangesInfo<'env> {
    environment_version: u32,
    changes_version: u32,
    changes: PendingProjectChanges<'env>,
}

struct EnvironmentHolder {
    environment: Option<Environment>,
    last_update: Option<tokio::time::Instant>,
    environment_version: Wrapping<u32>,
    last_repository_update: Option<tokio::time::Instant>,
}

impl EnvironmentHolder {
    fn new() -> Self {
        Self {
            environment: None,
            last_update: None,
            environment_version: Wrapping(0),
            last_repository_update: None,
        }
    }

    async fn get_environment_mut(
        &mut self,
        update_repository: UpdateRepositoryMode,
        io: &DefaultEnvironmentIo,
    ) -> io::Result<&mut Environment> {
        if let Some(ref mut environment) = self.environment {
            if !self
                .last_update
                .map(|x| x.elapsed() < tokio::time::Duration::from_secs(1))
                .unwrap_or(false)
            {
                info!("reloading settings files");
                // reload settings files
                environment.reload().await?;
                self.last_update = Some(tokio::time::Instant::now());
            }

            // outdated after 5 min
            const OUTDATED: tokio::time::Duration = tokio::time::Duration::from_secs(60 * 5);

            match update_repository {
                UpdateRepositoryMode::None => {}
                UpdateRepositoryMode::Force => {
                    self.last_repository_update = Some(tokio::time::Instant::now());
                    self.environment_version += Wrapping(1);
                    info!("loading package infos");
                    environment.load_package_infos(true).await?;
                }
                UpdateRepositoryMode::IfOutdatedOrNecessary => {
                    if self
                        .last_repository_update
                        .map(|x| x.elapsed() > OUTDATED)
                        .unwrap_or(true)
                    {
                        self.last_repository_update = Some(tokio::time::Instant::now());
                        self.environment_version += Wrapping(1);
                        info!("loading package infos");
                        environment.load_package_infos(true).await?;
                    }
                }
            }

            Ok(environment)
        } else {
            self.environment = Some(new_environment(io).await?);
            self.last_update = Some(tokio::time::Instant::now());
            let environment = self.environment.as_mut().unwrap();

            match update_repository {
                UpdateRepositoryMode::None => {}
                UpdateRepositoryMode::Force | UpdateRepositoryMode::IfOutdatedOrNecessary => {
                    self.last_repository_update = Some(tokio::time::Instant::now());
                    self.environment_version += Wrapping(1);
                    info!("loading package infos");
                    environment.load_package_infos(true).await?;
                }
            }

            Ok(environment)
        }
    }
}

enum UpdateRepositoryMode {
    None,
    Force,
    IfOutdatedOrNecessary,
}

struct ChangesInfoHolder {
    changes_info: Option<NonNull<PendingProjectChangesInfo<'static>>>,
}

impl ChangesInfoHolder {
    fn new() -> Self {
        Self { changes_info: None }
    }

    fn update(
        &mut self,
        environment_version: u32,
        changes: PendingProjectChanges<'_>,
    ) -> TauriPendingProjectChanges {
        static CHANGES_GLOBAL_INDEXER: AtomicU32 = AtomicU32::new(0);
        let changes_version = CHANGES_GLOBAL_INDEXER.fetch_add(1, Ordering::SeqCst);

        let result = TauriPendingProjectChanges::new(changes_version, &changes);

        let changes_info = Box::new(PendingProjectChangesInfo {
            environment_version,
            changes_version,
            changes,
        });

        if let Some(ptr) = self.changes_info.take() {
            unsafe { drop(Box::from_raw(ptr.as_ptr())) }
        }
        self.changes_info = NonNull::new(Box::into_raw(changes_info) as *mut _);

        result
    }

    fn take(&mut self) -> Option<PendingProjectChangesInfo> {
        Some(*unsafe { Box::from_raw(self.changes_info.take()?.as_mut()) })
    }
}

impl EnvironmentState {
    fn new(io: DefaultEnvironmentIo) -> Self {
        Self {
            environment: EnvironmentHolder::new(),
            config: GuiConfigHolder::new(),
            packages: None,
            projects: Box::new([]),
            projects_version: Wrapping(0),
            changes_info: ChangesInfoHolder::new(),
            io,
        }
    }
}

#[derive(Debug, Clone, Serialize, specta::Type)]
struct TauriProject {
    // the project identifier
    list_version: u32,
    index: usize,

    // projet information
    name: String,
    path: String,
    project_type: TauriProjectType,
    unity: String,
    unity_revision: Option<String>,
    last_modified: i64,
    created_at: i64,
    favorite: bool,
    is_exists: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
enum TauriProjectType {
    Unknown,
    LegacySdk2,
    LegacyWorlds,
    LegacyAvatars,
    UpmWorlds,
    UpmAvatars,
    UpmStarter,
    Worlds,
    Avatars,
    VpmStarter,
}

impl From<ProjectType> for TauriProjectType {
    fn from(value: ProjectType) -> Self {
        match value {
            ProjectType::Unknown => Self::Unknown,
            ProjectType::LegacySdk2 => Self::LegacySdk2,
            ProjectType::LegacyWorlds => Self::LegacyWorlds,
            ProjectType::LegacyAvatars => Self::LegacyAvatars,
            ProjectType::UpmWorlds => Self::UpmWorlds,
            ProjectType::UpmAvatars => Self::UpmAvatars,
            ProjectType::UpmStarter => Self::UpmStarter,
            ProjectType::Worlds => Self::Worlds,
            ProjectType::Avatars => Self::Avatars,
            ProjectType::VpmStarter => Self::VpmStarter,
        }
    }
}

impl TauriProject {
    fn new(list_version: u32, index: usize, project: &UserProject) -> Self {
        let is_exists = std::fs::metadata(project.path())
            .map(|x| x.is_dir())
            .unwrap_or(false);
        Self {
            list_version,
            index,

            name: project.name().to_string(),
            path: project.path().to_string(),
            project_type: project.project_type().into(),
            unity: project
                .unity_version()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown".into()),
            unity_revision: project.unity_revision().map(|x| x.to_string()),
            last_modified: project.last_modified().timestamp_millis(),
            created_at: project.crated_at().timestamp_millis(),
            favorite: project.favorite(),
            is_exists,
        }
    }
}

#[tauri::command]
#[specta::specta]
async fn environment_language(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<String, RustError> {
    with_config!(state, |config| Ok(config.language.clone()))
}

#[tauri::command]
#[specta::specta]
async fn environment_set_language(
    state: State<'_, Mutex<EnvironmentState>>,
    language: String,
) -> Result<(), RustError> {
    with_config!(state, |mut config| {
        config.language = language;
        config.save().await?;
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
async fn environment_theme(state: State<'_, Mutex<EnvironmentState>>) -> Result<String, RustError> {
    with_config!(state, |config| Ok(config.theme.clone()))
}

#[tauri::command]
#[specta::specta]
async fn environment_set_theme(
    state: State<'_, Mutex<EnvironmentState>>,
    theme: String,
) -> Result<(), RustError> {
    with_config!(state, |mut config| {
        config.theme = theme;
        config.save().await?;
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
async fn environment_projects(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<Vec<TauriProject>, RustError> {
    let mut state = state.lock().await;
    let state = &mut *state;
    let environment = state
        .environment
        .get_environment_mut(UpdateRepositoryMode::None, &state.io)
        .await?;

    info!("migrating projects from settings.json");
    // migrate from settings json
    environment.migrate_from_settings_json().await?;
    info!("syncing information with real projects");
    environment.sync_with_real_projects(true).await?;
    environment.save().await?;

    info!("fetching projects");

    let projects = environment.get_projects()?.into_boxed_slice();
    environment.disconnect_litedb();

    state.projects = projects;
    state.projects_version += Wrapping(1);

    let version = (state.environment.environment_version + state.projects_version).0;

    let vec = state
        .projects
        .iter()
        .enumerate()
        .map(|(index, value)| TauriProject::new(version, index, value))
        .collect::<Vec<_>>();

    Ok(vec)
}

#[derive(Serialize, specta::Type)]
enum TauriAddProjectWithPickerResult {
    NoFolderSelected,
    InvalidSelection,
    AlreadyAdded,
    Successful,
}

#[tauri::command]
#[specta::specta]
async fn environment_add_project_with_picker(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<TauriAddProjectWithPickerResult, RustError> {
    let Some(project_path) = FileDialogBuilder::new().pick_folder() else {
        return Ok(TauriAddProjectWithPickerResult::NoFolderSelected);
    };

    let Ok(project_path) = project_path.into_os_string().into_string() else {
        return Ok(TauriAddProjectWithPickerResult::InvalidSelection);
    };

    let unity_project = load_project(project_path.clone()).await?;
    if !unity_project.is_valid().await {
        return Ok(TauriAddProjectWithPickerResult::InvalidSelection);
    }

    with_environment!(&state, |environment| {
        let projects = environment.get_projects()?;
        if projects
            .iter()
            .any(|x| Path::new(x.path()) == Path::new(&project_path))
        {
            return Ok(TauriAddProjectWithPickerResult::AlreadyAdded);
        }
        environment.add_project(&unity_project).await?;
        environment.save().await?;
    });

    Ok(TauriAddProjectWithPickerResult::Successful)
}

async fn trash_delete(path: PathBuf) -> Result<(), trash::Error> {
    tokio::runtime::Handle::current()
        .spawn_blocking(move || trash::delete(path))
        .await
        .unwrap()
}

#[tauri::command]
#[specta::specta]
async fn environment_remove_project(
    state: State<'_, Mutex<EnvironmentState>>,
    list_version: u32,
    index: usize,
    directory: bool,
) -> Result<(), RustError> {
    let mut state = state.lock().await;
    let state = &mut *state;
    let version = (state.environment.environment_version + state.projects_version).0;
    if list_version != version {
        return Err(RustError::unrecoverable("project list version mismatch"));
    }

    let project = &state.projects[index];
    let environment = state
        .environment
        .get_environment_mut(UpdateRepositoryMode::None, &state.io)
        .await?;
    environment.remove_project(project)?;
    environment.save().await?;

    if directory {
        let path = project.path();
        info!("removing project directory: {path}");

        if let Err(err) = trash_delete(PathBuf::from(path)).await {
            error!("failed to remove project directory: {err}");
        } else {
            info!("removed project directory: {path}");
        }
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn environment_remove_project_by_path(
    state: State<'_, Mutex<EnvironmentState>>,
    path: String,
    directory: bool,
) -> Result<(), RustError> {
    with_environment!(&state, |environment| {
        let projects: Vec<vrc_get_vpm::environment::UserProject> = environment.get_projects()?;

        if let Some(x) = projects.iter().find(|x| x.path() == path) {
            environment.remove_project(x)?;
            environment.save().await?;
        } else {
            environment.disconnect_litedb();
        }

        if directory {
            info!("removing project directory: {path}");
            if let Err(err) = trash_delete(PathBuf::from(&path)).await {
                error!("failed to remove project directory: {err}");
            } else {
                info!("removed project directory: {path}");
            }
        }

        Ok(())
    })
}

async fn copy_recursively(from: PathBuf, to: PathBuf) -> fs_extra::error::Result<u64> {
    let mut options = fs_extra::dir::CopyOptions::new();
    options.copy_inside = false;
    options.content_only = true;
    match tokio::runtime::Handle::current()
        .spawn_blocking(move || fs_extra::dir::copy(from, to, &options))
        .await
    {
        Ok(r) => Ok(r?),
        Err(_) => Err(io::Error::new(io::ErrorKind::Other, "background task failed").into()),
    }
}

#[tauri::command]
#[specta::specta]
async fn environment_copy_project_for_migration(
    state: State<'_, Mutex<EnvironmentState>>,
    source_path: String,
) -> Result<String, RustError> {
    async fn create_folder(folder: &Path, name: &OsStr) -> Option<String> {
        let name = name.to_str().unwrap();
        // first, try `-Migrated`
        let new_path = folder.join(format!("{name}-Migrated"));
        if let Ok(()) = tokio::fs::create_dir(&new_path).await {
            return Some(new_path.into_os_string().into_string().unwrap());
        }

        for i in 1..100 {
            let new_path = folder.join(format!("{name}-Migrated-{i}"));
            if let Ok(()) = tokio::fs::create_dir(&new_path).await {
                return Some(new_path.into_os_string().into_string().unwrap());
            }
        }

        None
    }

    let source_path_str = source_path;
    let source_path = Path::new(&source_path_str);
    let folder = source_path.parent().unwrap();
    let name = source_path.file_name().unwrap();

    let Some(new_path_str) = create_folder(folder, name).await else {
        return Err(RustError::unrecoverable(
            "failed to create a new folder for migration",
        ));
    };
    let new_path = Path::new(&new_path_str);

    info!("copying project for migration: {source_path_str} -> {new_path_str}");

    let mut source_path_read = read_dir(source_path).await?;
    while let Some(entry) = source_path_read.next_entry().await? {
        if entry.file_name().to_ascii_lowercase() == "library"
            || entry.file_name().to_ascii_lowercase() == "temp"
        {
            continue;
        }

        if entry.file_type().await?.is_dir() {
            copy_recursively(entry.path(), new_path.join(entry.file_name()))
                .await
                .map_err(|e| format!("copying {}: {e}", entry.path().display()))?;
        } else {
            tokio::fs::copy(entry.path(), new_path.join(entry.file_name()))
                .await
                .map_err(|e| format!("copying {}: {e}", entry.path().display()))?;
        }
    }

    info!("copied project for migration. adding to listing");

    let unity_project = load_project(new_path_str.clone()).await?;

    with_environment!(state, |environment| {
        environment.add_project(&unity_project).await?;
        environment.save().await?;
    });

    Ok(new_path_str)
}

#[tauri::command]
#[specta::specta]
async fn environment_set_favorite_project(
    state: State<'_, Mutex<EnvironmentState>>,
    list_version: u32,
    index: usize,
    favorite: bool,
) -> Result<(), RustError> {
    let mut state = state.lock().await;
    let state = &mut *state;
    let version = (state.environment.environment_version + state.projects_version).0;
    if list_version != version {
        return Err(RustError::unrecoverable("project list version mismatch"));
    }

    let project = &mut state.projects[index];
    project.set_favorite(favorite);
    let environment = state
        .environment
        .get_environment_mut(UpdateRepositoryMode::None, &state.io)
        .await?;
    environment.update_project(project)?;
    environment.save().await?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn environment_get_project_sorting(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<String, RustError> {
    with_config!(state, |config| Ok(config.project_sorting.clone()))
}

#[tauri::command]
#[specta::specta]
async fn environment_set_project_sorting(
    state: State<'_, Mutex<EnvironmentState>>,
    sorting: String,
) -> Result<(), RustError> {
    with_config!(state, |mut config| {
        config.project_sorting = sorting;
        config.save().await?;
        Ok(())
    })
}

#[derive(Serialize, specta::Type)]
struct TauriVersion {
    major: u64,
    minor: u64,
    patch: u64,
    pre: String,
    build: String,
}

impl From<&Version> for TauriVersion {
    fn from(value: &Version) -> Self {
        Self {
            major: value.major,
            minor: value.minor,
            patch: value.patch,
            pre: value.pre.as_str().to_string(),
            build: value.build.as_str().to_string(),
        }
    }
}

#[derive(Serialize, specta::Type)]
struct TauriBasePackageInfo {
    name: String,
    display_name: Option<String>,
    aliases: Vec<String>,
    version: TauriVersion,
    unity: Option<(u16, u8)>,
    changelog_url: Option<String>,
    vpm_dependencies: Vec<String>,
    legacy_packages: Vec<String>,
    is_yanked: bool,
}

impl TauriBasePackageInfo {
    fn new(package: &PackageManifest) -> Self {
        Self {
            name: package.name().to_string(),
            display_name: package.display_name().map(|v| v.to_string()),
            aliases: package.aliases().iter().map(|v| v.to_string()).collect(),
            version: package.version().into(),
            unity: package.unity().map(|v| (v.major(), v.minor())),
            changelog_url: package.changelog_url().map(|v| v.to_string()),
            vpm_dependencies: package
                .vpm_dependencies()
                .keys()
                .map(|x| x.to_string())
                .collect(),
            legacy_packages: package
                .legacy_packages()
                .iter()
                .map(|x| x.to_string())
                .collect(),
            is_yanked: package.is_yanked(),
        }
    }
}

#[derive(Serialize, specta::Type)]
struct TauriPackage {
    env_version: u32,
    index: usize,

    #[serde(flatten)]
    base: TauriBasePackageInfo,

    source: TauriPackageSource,
}

#[derive(Serialize, specta::Type)]
enum TauriPackageSource {
    LocalUser,
    Remote { id: String, display_name: String },
}

impl TauriPackage {
    fn new(env_version: u32, index: usize, package: &PackageInfo) -> Self {
        let source = if let Some(repo) = package.repo() {
            let id = repo.id().or(repo.url().map(|x| x.as_str())).unwrap();
            TauriPackageSource::Remote {
                id: id.to_string(),
                display_name: repo.name().unwrap_or(id).to_string(),
            }
        } else {
            TauriPackageSource::LocalUser
        };

        Self {
            env_version,
            index,
            base: TauriBasePackageInfo::new(package.package_json()),
            source,
        }
    }
}

#[tauri::command]
#[specta::specta]
async fn environment_refetch_packages(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<(), RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    env_state
        .environment
        .get_environment_mut(UpdateRepositoryMode::Force, &env_state.io)
        .await?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn environment_packages(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<Vec<TauriPackage>, RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    let environment = env_state
        .environment
        .get_environment_mut(UpdateRepositoryMode::IfOutdatedOrNecessary, &env_state.io)
        .await?;

    let packages = environment
        .get_all_packages()
        .collect::<Vec<_>>()
        .into_boxed_slice();
    if let Some(ptr) = env_state.packages {
        env_state.packages = None; // avoid a double drop
        unsafe { drop(Box::from_raw(ptr.as_ptr())) }
    }
    env_state.packages = NonNull::new(Box::into_raw(packages) as *mut _);
    let packages = unsafe { &*env_state.packages.unwrap().as_ptr() };
    let version = env_state.environment.environment_version.0;

    Ok(packages
        .iter()
        .enumerate()
        .map(|(index, value)| TauriPackage::new(version, index, value))
        .collect::<Vec<_>>())
}

#[derive(Serialize, specta::Type)]
struct TauriUserRepository {
    id: String,
    url: Option<String>,
    display_name: String,
}

#[derive(Serialize, specta::Type)]
struct TauriRepositoriesInfo {
    user_repositories: Vec<TauriUserRepository>,
    hidden_user_repositories: Vec<String>,
    hide_local_user_packages: bool,
    show_prerelease_packages: bool,
}

#[tauri::command]
#[specta::specta]
async fn environment_repositories_info(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<TauriRepositoriesInfo, RustError> {
    with_environment!(&state, |environment, config| {
        Ok(TauriRepositoriesInfo {
            user_repositories: environment
                .get_user_repos()
                .iter()
                .map(|x| {
                    let id = x.id().or(x.url().map(Url::as_str)).unwrap();
                    TauriUserRepository {
                        id: id.to_string(),
                        url: x.url().map(|x| x.to_string()),
                        display_name: x.name().unwrap_or(id).to_string(),
                    }
                })
                .collect(),
            hidden_user_repositories: config.gui_hidden_repositories.iter().cloned().collect(),
            hide_local_user_packages: config.hide_local_user_packages,
            show_prerelease_packages: environment.show_prerelease_packages(),
        })
    })
}

#[tauri::command]
#[specta::specta]
async fn environment_hide_repository(
    state: State<'_, Mutex<EnvironmentState>>,
    repository: String,
) -> Result<(), RustError> {
    with_config!(&state, |mut config| {
        config.gui_hidden_repositories.insert(repository);
        config.save().await?;
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
async fn environment_show_repository(
    state: State<'_, Mutex<EnvironmentState>>,
    repository: String,
) -> Result<(), RustError> {
    with_config!(&state, |mut config| {
        config.gui_hidden_repositories.shift_remove(&repository);
        config.save().await?;
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
async fn environment_set_hide_local_user_packages(
    state: State<'_, Mutex<EnvironmentState>>,
    value: bool,
) -> Result<(), RustError> {
    with_environment!(&state, |_, mut config| {
        config.hide_local_user_packages = value;
        config.save().await?;
        Ok(())
    })
}

#[derive(Serialize, specta::Type)]
struct TauriUnityVersions {
    unity_paths: Vec<(String, String, bool)>,
    recommended_version: String,
    install_recommended_version_link: String,
}

#[tauri::command]
#[specta::specta]
async fn environment_unity_versions(
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
struct TauriEnvironmentSettings {
    default_project_path: String,
    project_backup_path: String,
    unity_hub: String,
    unity_paths: Vec<(String, String, bool)>,
    show_prerelease_packages: bool,
    backup_format: String,
}

#[tauri::command]
#[specta::specta]
async fn environment_get_settings(
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
enum TauriPickUnityHubResult {
    NoFolderSelected,
    InvalidSelection,
    Successful,
}

#[tauri::command]
#[specta::specta]
async fn environment_pick_unity_hub(
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
enum TauriPickUnityResult {
    NoFolderSelected,
    InvalidSelection,
    AlreadyAdded,
    Successful,
}

#[tauri::command]
#[specta::specta]
async fn environment_pick_unity(
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
enum TauriPickProjectDefaultPathResult {
    NoFolderSelected,
    InvalidSelection,
    Successful { new_path: String },
}

#[tauri::command]
#[specta::specta]
async fn environment_pick_project_default_path(
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
enum TauriPickProjectBackupPathResult {
    NoFolderSelected,
    InvalidSelection,
    Successful,
}

#[tauri::command]
#[specta::specta]
async fn environment_pick_project_backup_path(
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
async fn environment_set_show_prerelease_packages(
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
async fn environment_set_backup_format(
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

#[derive(Serialize, specta::Type)]
struct TauriRemoteRepositoryInfo {
    display_name: String,
    id: String,
    url: String,
    packages: Vec<TauriBasePackageInfo>,
}

#[derive(Serialize, specta::Type)]
#[serde(tag = "type")]
enum TauriDownloadRepository {
    BadUrl,
    Duplicated,
    DownloadError { message: String },
    Success { value: TauriRemoteRepositoryInfo },
}

// workaround IndexMap v2 is not implemented in specta

#[tauri::command]
#[specta::specta]
async fn environment_download_repository(
    state: State<'_, Mutex<EnvironmentState>>,
    url: String,
    headers: IndexMapV2<Box<str>, Box<str>>,
) -> Result<TauriDownloadRepository, RustError> {
    let url: Url = match url.parse() {
        Err(_) => {
            return Ok(TauriDownloadRepository::BadUrl);
        }
        Ok(url) => url,
    };

    with_environment!(state, |environment| {
        for repo in environment.get_user_repos() {
            if repo.url().map(|x| x.as_str()) == Some(url.as_str()) {
                return Ok(TauriDownloadRepository::Duplicated);
            }
        }

        let client = environment.http().unwrap();
        let repo = match RemoteRepository::download(client, &url, &headers.0).await {
            Ok((repo, _)) => repo,
            Err(e) => {
                return Ok(TauriDownloadRepository::DownloadError {
                    message: e.to_string(),
                });
            }
        };

        let url = repo.url().unwrap_or(&url).as_str();
        let id = repo.id().unwrap_or(url);

        for repo in environment.get_user_repos() {
            if repo.id() == Some(id) {
                return Ok(TauriDownloadRepository::Duplicated);
            }
        }

        Ok(TauriDownloadRepository::Success {
            value: TauriRemoteRepositoryInfo {
                id: id.to_string(),
                url: url.to_string(),
                display_name: repo.name().unwrap_or(id).to_string(),
                packages: repo
                    .get_packages()
                    .filter_map(|x| x.get_latest(VersionSelector::latest_for(None, true)))
                    .filter(|x| !x.is_yanked())
                    .map(TauriBasePackageInfo::new)
                    .collect(),
            },
        })
    })
}

#[derive(Serialize, specta::Type)]
enum TauriAddRepositoryResult {
    BadUrl,
    Success,
}

#[tauri::command]
#[specta::specta]
async fn environment_add_repository(
    state: State<'_, Mutex<EnvironmentState>>,
    url: String,
    headers: IndexMapV2<Box<str>, Box<str>>,
) -> Result<TauriAddRepositoryResult, RustError> {
    let url: Url = match url.parse() {
        Err(_) => {
            return Ok(TauriAddRepositoryResult::BadUrl);
        }
        Ok(url) => url,
    };

    with_environment!(&state, |environment| {
        environment.add_remote_repo(url, None, headers.0).await?;
        environment.save().await?;
    });

    // force update repository
    let mut state = state.lock().await;
    state.environment.last_repository_update = None;

    Ok(TauriAddRepositoryResult::Success)
}

#[tauri::command]
#[specta::specta]
async fn environment_remove_repository(
    state: State<'_, Mutex<EnvironmentState>>,
    id: String,
) -> Result<(), RustError> {
    with_environment!(state, |environment| {
        environment
            .remove_repo(|r| r.id() == Some(id.as_str()))
            .await;

        environment.save().await?;
    });

    Ok(())
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
enum TauriProjectTemplate {
    Builtin { id: String, name: String },
    Custom { name: String },
}

#[derive(Serialize, specta::Type)]
struct TauriProjectCreationInformation {
    templates: Vec<TauriProjectTemplate>,
    default_path: String,
}

async fn load_user_templates(environment: &mut Environment) -> io::Result<Vec<String>> {
    let io = environment.io();

    let mut templates = Vec::<String>::new();

    let path = io.resolve("Templates".as_ref());
    let mut dir = io.read_dir("Templates".as_ref()).await?;
    while let Some(dir) = dir.try_next().await? {
        if !dir.file_type().await?.is_dir() {
            continue;
        }

        let Ok(name) = dir.file_name().into_string() else {
            continue;
        };

        let path = path.join(&name);

        // check package.json
        let Ok(pkg_json) = tokio::fs::metadata(path.join("package.json")).await else {
            continue;
        };
        if !pkg_json.is_file() {
            continue;
        }

        match UnityProject::load(DefaultProjectIo::new(path.into())).await {
            Err(e) => {
                warn!("failed to load user template {name}: {e}");
            }
            Ok(ref p) if !p.is_valid().await => {
                warn!("failed to load user template {name}: invalid project");
            }
            Ok(_) => {}
        }

        templates.push(name)
    }

    Ok(templates)
}

#[tauri::command]
#[specta::specta]
async fn environment_project_creation_information(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<TauriProjectCreationInformation, RustError> {
    with_environment!(state, |environment| {
        let mut templates = crate::templates::TEMPLATES
            .iter()
            .map(|&(id, name, _)| TauriProjectTemplate::Builtin {
                id: id.into(),
                name: name.into(),
            })
            .collect::<Vec<_>>();

        templates.extend(
            load_user_templates(environment)
                .await
                .ok()
                .into_iter()
                .flatten()
                .map(|name| TauriProjectTemplate::Custom { name }),
        );

        Ok(TauriProjectCreationInformation {
            templates,
            default_path: environment.default_project_path().to_string(),
        })
    })
}

#[derive(Serialize, specta::Type)]
enum TauriProjectDirCheckResult {
    // path related
    InvalidNameForFolderName,
    MayCompatibilityProblem,
    WideChar,

    AlreadyExists,
    Ok,
}

static WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
    "COM8", "COM9", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

static WINDOWS_RESERVED_CHARS: &[char] = &['/', '\\', '<', '>', ':', '"', '|', '?', '*'];

#[tauri::command]
#[specta::specta]
async fn environment_check_project_name(
    base_path: String,
    project_name: String,
) -> Result<TauriProjectDirCheckResult, RustError> {
    let project_name = project_name.trim();
    let project_name_upper = project_name.to_ascii_uppercase();

    if project_name.is_empty()
        || project_name.len() > 255
        || WINDOWS_RESERVED_NAMES.contains(&project_name_upper.as_str())
        || project_name.contains(WINDOWS_RESERVED_CHARS)
    {
        return Ok(TauriProjectDirCheckResult::InvalidNameForFolderName);
    }

    let path = Path::new(&base_path).join(project_name);
    if path.exists() {
        return Ok(TauriProjectDirCheckResult::AlreadyExists);
    }

    if cfg!(target_os = "windows") {
        if project_name.contains('%') {
            return Ok(TauriProjectDirCheckResult::MayCompatibilityProblem);
        }

        if project_name.chars().any(|c| c as u32 > 0x7F) {
            return Ok(TauriProjectDirCheckResult::WideChar);
        }
    }

    Ok(TauriProjectDirCheckResult::Ok)
}

#[derive(Serialize, specta::Type)]
enum TauriCreateProjectResult {
    AlreadyExists,
    TemplateNotFound,
    Successful,
}

#[tauri::command]
#[specta::specta]
async fn environment_create_project(
    state: State<'_, Mutex<EnvironmentState>>,
    base_path: String,
    project_name: String,
    template: TauriProjectTemplate,
) -> Result<TauriCreateProjectResult, RustError> {
    enum Template {
        Builtin(&'static [u8]),
        Custom(PathBuf),
    }

    // first, check the template.
    let template = match template {
        TauriProjectTemplate::Builtin { id, .. } => {
            let Some((_, _, template)) = crate::templates::TEMPLATES.iter().find(|x| x.0 == id)
            else {
                return Ok(TauriCreateProjectResult::TemplateNotFound);
            };
            Template::Builtin(template)
        }
        TauriProjectTemplate::Custom { name } => {
            let template_path = with_environment!(state, |enviornment| {
                enviornment
                    .io()
                    .resolve(format!("Templates/{name}").as_ref())
            });
            match tokio::fs::metadata(&template_path).await {
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                    return Ok(TauriCreateProjectResult::TemplateNotFound);
                }
                Err(e) => {
                    return Err(e.into());
                }
                Ok(ref meta) if !meta.is_dir() => {
                    return Ok(TauriCreateProjectResult::TemplateNotFound);
                }
                Ok(_) => {}
            }
            Template::Custom(template_path)
        }
    };

    let base_path = Path::new(&base_path);
    let path = base_path.join(&project_name);
    let path_str = path.to_str().unwrap();

    // we split creating folder into two phases
    // because we want to fail if the project folder already exists.

    // create parent directory if not exists (unlikely to happen)
    tokio::fs::create_dir_all(base_path).await?;

    // create project directory
    match tokio::fs::create_dir(&path).await {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            return Ok(TauriCreateProjectResult::AlreadyExists);
        }
        Err(e) => {
            return Err(e.into());
        }
    }

    // copy template contents to the project directory
    match template {
        Template::Builtin(tgz) => {
            let tar = flate2::read::GzDecoder::new(std::io::Cursor::new(tgz));
            let mut archive = tar::Archive::new(tar);
            archive.unpack(&path)?;
        }
        Template::Custom(template) => {
            copy_recursively(template, path.clone()).await?;
            // remove unnecessary package.json and README.md
            tokio::fs::remove_file(path.join("package.json")).await.ok();
            tokio::fs::remove_file(path.join("README.md")).await.ok();
        }
    }

    // update ProjectSettings.asset
    {
        let settings_path = path.join("ProjectSettings/ProjectSettings.asset");
        let mut settings_file = tokio::fs::File::options()
            .read(true)
            .write(true)
            .open(&settings_path)
            .await?;

        let mut settings = String::new();
        settings_file.read_to_string(&mut settings).await?;

        fn set_value(buffer: &mut String, finder: &str, value: &str) {
            if let Some(pos) = buffer.find(finder) {
                let before_ws = buffer[..pos]
                    .chars()
                    .last()
                    .map(|x| x.is_ascii_whitespace())
                    .unwrap_or(true);
                if before_ws {
                    if let Some(eol) = buffer[pos..].find('\n') {
                        let eol = eol + pos;
                        buffer.replace_range((pos + finder.len())..eol, value);
                    }
                }
            }
        }

        fn yaml_quote(value: &str) -> String {
            let s = value
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r");
            format!("\"{}\"", s)
        }

        set_value(
            &mut settings,
            "productGUID: ",
            &uuid::Uuid::new_v4().simple().to_string(),
        );
        set_value(&mut settings, "productName: ", &yaml_quote(&project_name));

        settings_file.seek(std::io::SeekFrom::Start(0)).await?;
        settings_file.set_len(0).await?;
        settings_file.write_all(settings.as_bytes()).await?;
        settings_file.flush().await?;
        drop(settings_file);
    }

    {
        let mut env_state = state.lock().await;
        let env_state = &mut *env_state;
        let environment = env_state
            .environment
            .get_environment_mut(UpdateRepositoryMode::IfOutdatedOrNecessary, &env_state.io)
            .await?;

        let mut unity_project = load_project(path_str.into()).await?;

        // finally, resolve the project folder
        let request = unity_project.resolve_request(environment).await?;
        unity_project
            .apply_pending_changes(environment, request)
            .await?;
        unity_project.save().await?;

        // add the project to listing
        environment.add_project(&unity_project).await?;
        environment.save().await?;
    }
    Ok(TauriCreateProjectResult::Successful)
}

#[derive(Serialize, specta::Type)]
struct TauriProjectDetails {
    unity: Option<(u16, u8)>,
    unity_str: Option<String>,
    unity_revision: Option<String>,
    installed_packages: Vec<(String, TauriBasePackageInfo)>,
    should_resolve: bool,
}

async fn load_project(project_path: String) -> Result<UnityProject, RustError> {
    Ok(UnityProject::load(vrc_get_vpm::io::DefaultProjectIo::new(
        PathBuf::from(project_path).into(),
    ))
    .await?)
}

#[tauri::command]
#[specta::specta]
async fn project_details(project_path: String) -> Result<TauriProjectDetails, RustError> {
    let unity_project = load_project(project_path).await?;

    Ok(TauriProjectDetails {
        unity: unity_project
            .unity_version()
            .map(|v| (v.major(), v.minor())),
        unity_str: unity_project.unity_version().map(|v| v.to_string()),
        unity_revision: unity_project.unity_revision().map(|x| x.to_string()),
        installed_packages: unity_project
            .installed_packages()
            .map(|(k, p)| (k.to_string(), TauriBasePackageInfo::new(p)))
            .collect(),
        should_resolve: unity_project.should_resolve(),
    })
}

#[derive(Serialize, specta::Type)]
struct TauriPendingProjectChanges {
    changes_version: u32,
    package_changes: Vec<(String, TauriPackageChange)>,

    remove_legacy_files: Vec<String>,
    remove_legacy_folders: Vec<String>,

    conflicts: Vec<(String, TauriConflictInfo)>,
}

impl TauriPendingProjectChanges {
    fn new(version: u32, changes: &PendingProjectChanges) -> Self {
        TauriPendingProjectChanges {
            changes_version: version,
            package_changes: changes
                .package_changes()
                .iter()
                .filter_map(|(name, change)| Some((name.to_string(), change.try_into().ok()?)))
                .collect(),
            remove_legacy_files: changes
                .remove_legacy_files()
                .iter()
                .map(|(x, _)| x.to_string_lossy().into_owned())
                .collect(),
            remove_legacy_folders: changes
                .remove_legacy_folders()
                .iter()
                .map(|(x, _)| x.to_string_lossy().into_owned())
                .collect(),
            conflicts: changes
                .conflicts()
                .iter()
                .map(|(name, info)| (name.to_string(), info.into()))
                .collect(),
        }
    }
}

#[derive(Serialize, specta::Type)]
enum TauriPackageChange {
    InstallNew(Box<TauriBasePackageInfo>),
    Remove(TauriRemoveReason),
}

impl TryFrom<&PackageChange<'_>> for TauriPackageChange {
    type Error = ();

    fn try_from(value: &PackageChange) -> Result<Self, ()> {
        Ok(match value {
            PackageChange::Install(install) => TauriPackageChange::InstallNew(
                TauriBasePackageInfo::new(install.install_package().ok_or(())?.package_json())
                    .into(),
            ),
            PackageChange::Remove(remove) => TauriPackageChange::Remove(remove.reason().into()),
        })
    }
}

#[derive(Serialize, specta::Type)]
enum TauriRemoveReason {
    Requested,
    Legacy,
    Unused,
}

impl From<RemoveReason> for TauriRemoveReason {
    fn from(value: RemoveReason) -> Self {
        match value {
            RemoveReason::Requested => Self::Requested,
            RemoveReason::Legacy => Self::Legacy,
            RemoveReason::Unused => Self::Unused,
        }
    }
}

#[derive(Serialize, specta::Type)]
struct TauriConflictInfo {
    packages: Vec<String>,
    unity_conflict: bool,
}

impl From<&ConflictInfo> for TauriConflictInfo {
    fn from(value: &ConflictInfo) -> Self {
        Self {
            packages: value
                .conflicting_packages()
                .iter()
                .map(|x| x.to_string())
                .collect(),
            unity_conflict: value.conflicts_with_unity(),
        }
    }
}

macro_rules! changes {
    ($state: ident, $($env_version: ident, )? |$environment: pat_param, $packages: pat_param| $body: expr) => {{
        let mut state = $state.lock().await;
        let state = &mut *state;
        let current_version = state.environment.environment_version.0;
        $(
        if current_version != $env_version {
            return Err(RustError::unrecoverable("environment version mismatch"));
        }
        )?

        let $environment = state.environment.get_environment_mut(UpdateRepositoryMode::None, &state.io).await?;
        let $packages = unsafe { &*state.packages.unwrap().as_mut() };
        let changes = $body;

        Ok(state.changes_info.update(current_version, changes))
    }};
}

#[tauri::command]
#[specta::specta]
async fn project_install_package(
    state: State<'_, Mutex<EnvironmentState>>,
    project_path: String,
    env_version: u32,
    package_index: usize,
) -> Result<TauriPendingProjectChanges, RustError> {
    changes!(state, env_version, |environment, packages| {
        let installing_package = packages[package_index];

        let unity_project = load_project(project_path).await?;

        let operation = if let Some(locked) = unity_project.get_locked(installing_package.name()) {
            if installing_package.version() < locked.version() {
                AddPackageOperation::Downgrade
            } else {
                AddPackageOperation::UpgradeLocked
            }
        } else {
            AddPackageOperation::InstallToDependencies
        };

        let allow_prerelease = environment.show_prerelease_packages();

        match unity_project
            .add_package_request(
                environment,
                &[installing_package],
                operation,
                allow_prerelease,
            )
            .await
        {
            Ok(request) => request,
            Err(e) => return Err(RustError::unrecoverable(e)),
        }
    })
}

#[tauri::command]
#[specta::specta]
async fn project_install_multiple_package(
    state: State<'_, Mutex<EnvironmentState>>,
    project_path: String,
    env_version: u32,
    package_indices: Vec<usize>,
) -> Result<TauriPendingProjectChanges, RustError> {
    changes!(state, env_version, |environment, packages| {
        let installing_packages = package_indices
            .iter()
            .map(|index| packages[*index])
            .collect::<Vec<_>>();

        let unity_project = load_project(project_path).await?;

        let operation = AddPackageOperation::InstallToDependencies;

        let allow_prerelease = environment.show_prerelease_packages();

        match unity_project
            .add_package_request(
                environment,
                &installing_packages,
                operation,
                allow_prerelease,
            )
            .await
        {
            Ok(request) => request,
            Err(e) => return Err(RustError::unrecoverable(e)),
        }
    })
}

#[tauri::command]
#[specta::specta]
async fn project_upgrade_multiple_package(
    state: State<'_, Mutex<EnvironmentState>>,
    project_path: String,
    env_version: u32,
    package_indices: Vec<usize>,
) -> Result<TauriPendingProjectChanges, RustError> {
    changes!(state, env_version, |environment, packages| {
        let installing_packages = package_indices
            .iter()
            .map(|index| packages[*index])
            .collect::<Vec<_>>();

        let unity_project = load_project(project_path).await?;

        let operation = AddPackageOperation::UpgradeLocked;

        let allow_prerelease = environment.show_prerelease_packages();

        match unity_project
            .add_package_request(
                environment,
                &installing_packages,
                operation,
                allow_prerelease,
            )
            .await
        {
            Ok(request) => request,
            Err(e) => return Err(RustError::unrecoverable(e)),
        }
    })
}

#[tauri::command]
#[specta::specta]
async fn project_resolve(
    state: State<'_, Mutex<EnvironmentState>>,
    project_path: String,
) -> Result<TauriPendingProjectChanges, RustError> {
    changes!(state, |environment, _| {
        let unity_project = load_project(project_path).await?;

        match unity_project.resolve_request(environment).await {
            Ok(request) => request,
            Err(e) => return Err(RustError::unrecoverable(e)),
        }
    })
}

#[tauri::command]
#[specta::specta]
async fn project_remove_packages(
    state: State<'_, Mutex<EnvironmentState>>,
    project_path: String,
    names: Vec<String>,
) -> Result<TauriPendingProjectChanges, RustError> {
    changes!(state, |_, _| {
        let unity_project = load_project(project_path).await?;

        let names = names.iter().map(|x| x.as_str()).collect::<Vec<_>>();

        match unity_project.remove_request(&names).await {
            Ok(request) => request,
            Err(e) => return Err(RustError::unrecoverable(e)),
        }
    })
}

#[tauri::command]
#[specta::specta]
async fn project_apply_pending_changes(
    state: State<'_, Mutex<EnvironmentState>>,
    project_path: String,
    changes_version: u32,
) -> Result<(), RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    let changes = env_state.changes_info.take().unwrap();
    if changes.changes_version != changes_version {
        return Err(RustError::unrecoverable("changes version mismatch"));
    }
    if changes.environment_version != env_state.environment.environment_version.0 {
        return Err(RustError::unrecoverable("environment version mismatch"));
    }

    let environment = env_state
        .environment
        .get_environment_mut(UpdateRepositoryMode::None, &env_state.io)
        .await?;

    let mut unity_project = load_project(project_path).await?;

    unity_project
        .apply_pending_changes(environment, changes.changes)
        .await?;

    unity_project.save().await?;
    update_project_last_modified(environment, unity_project.project_dir()).await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn project_migrate_project_to_2022(
    state: State<'_, Mutex<EnvironmentState>>,
    project_path: String,
) -> Result<(), RustError> {
    with_environment!(state, |environment| {
        let mut unity_project = load_project(project_path).await?;

        match unity_project.migrate_unity_2022(environment).await {
            Ok(()) => {}
            Err(e) => return Err(RustError::unrecoverable(e)),
        }

        unity_project.save().await?;
        update_project_last_modified(environment, unity_project.project_dir()).await;

        Ok(())
    })
}

#[derive(Serialize, specta::Type, Clone)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum TauriCallUnityForMigrationResult {
    ExistsWithNonZero { status: String },
    FinishedSuccessfully,
}

#[allow(dead_code)]
#[tauri::command]
#[specta::specta]
async fn project_call_unity_for_migration(
    window: Window,
    channel: String,
    project_path: String,
    unity_path: String,
) -> Result<AsyncCallResult<String, TauriCallUnityForMigrationResult>, RustError> {
    async_command(channel, window, async {
        let unity_project = load_project(project_path).await?;

        With::<String>::continue_async(move |context| async move {
            let mut child = Command::new(unity_path)
                .args([
                    "-quit".as_ref(),
                    "-batchmode".as_ref(),
                    "-ignorecompilererrors".as_ref(),
                    // https://docs.unity3d.com/Manual/EditorCommandLineArguments.html
                    "-logFile".as_ref(),
                    "-".as_ref(),
                    "-projectPath".as_ref(),
                    unity_project.project_dir().as_os_str(),
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null())
                .spawn()?;

            // stdout and stderr
            tokio::spawn(send_lines(child.stdout.take().unwrap(), context.clone()));
            tokio::spawn(send_lines(child.stderr.take().unwrap(), context.clone()));

            // process end
            let status = child.wait().await?;

            return if status.success() {
                Ok(TauriCallUnityForMigrationResult::FinishedSuccessfully)
            } else {
                Ok(TauriCallUnityForMigrationResult::ExistsWithNonZero {
                    status: status.to_string(),
                })
            };

            async fn send_lines(
                stdout: impl tokio::io::AsyncRead + Unpin,
                context: AsyncCommandContext<String>,
            ) {
                let stdout = BufReader::new(stdout);
                let mut stdout = stdout.lines();
                loop {
                    match stdout.next_line().await {
                        Err(e) => {
                            error!("error reading unity output: {e}");
                            break;
                        }
                        Ok(None) => break,
                        Ok(Some(line)) => {
                            log::debug!(target: "vrc_get_gui::unity", "{line}");
                            let line = line.trim().to_string();
                            if let Err(e) = context.emit(line) {
                                error!("error sending stdout: {e}")
                            }
                        }
                    }
                }
            }
        })
    })
    .await
}

#[tauri::command]
#[specta::specta]
async fn project_migrate_project_to_vpm(
    state: State<'_, Mutex<EnvironmentState>>,
    project_path: String,
) -> Result<(), RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    let environment = env_state
        .environment
        .get_environment_mut(UpdateRepositoryMode::IfOutdatedOrNecessary, &env_state.io)
        .await?;

    let mut unity_project = load_project(project_path).await?;

    match unity_project
        .migrate_vpm(environment, environment.show_prerelease_packages())
        .await
    {
        Ok(()) => {}
        Err(e) => return Err(RustError::unrecoverable(e)),
    }

    unity_project.save().await?;
    update_project_last_modified(environment, unity_project.project_dir()).await;

    Ok(())
}

fn is_unity_running(project_path: impl AsRef<Path>) -> bool {
    crate::os::is_locked(&project_path.as_ref().join("Temp/UnityLockFile")).unwrap_or(false)
}

#[tauri::command]
#[specta::specta]
async fn project_open_unity(
    state: State<'_, Mutex<EnvironmentState>>,
    project_path: String,
    unity_path: String,
) -> Result<bool, RustError> {
    if is_unity_running(&project_path) {
        // it looks unity is running. returning false
        return Ok(false);
    }

    with_environment!(&state, |environment| {
        update_project_last_modified(environment, project_path.as_ref()).await;
    });

    crate::os::start_command(
        "Unity".as_ref(),
        unity_path.as_ref(),
        &["-projectPath".as_ref(), OsStr::new(project_path.as_str())],
    )
    .await?;

    Ok(true)
}

#[tauri::command]
#[specta::specta]
fn project_is_unity_launching(project_path: String) -> bool {
    return is_unity_running(&project_path);
}

fn folder_stream(
    path_buf: PathBuf,
) -> impl Stream<Item = io::Result<(String, tokio::fs::DirEntry)>> {
    async_stream::stream! {
        let mut stack = Vec::new();
        stack.push((String::from(""), tokio::fs::read_dir(&path_buf).await?));

        while let Some((dir, read_dir)) = stack.last_mut() {
            if let Some(entry) = read_dir.next_entry().await? {
                let Ok(file_name) = entry.file_name().into_string() else {
                    // non-utf8 file name
                    warn!("skipping non-utf8 file name: {}", entry.path().display());
                    continue;
                };
                log::trace!("process: {dir}{file_name}");

                if entry.file_type().await?.is_dir() {
                    let lower_name = file_name.to_ascii_lowercase();
                    if dir.is_empty() {
                        match lower_name.as_str() {
                            "library" | "logs" | "obj" | "temp" => {
                                continue;
                            }
                            lower_name => {
                                // some people uses multple library folder to speed up switch platform
                                if lower_name.starts_with("library") {
                                    continue;
                                }
                            }
                        }
                    }
                    if lower_name.as_str() == ".git" {
                        // any .git folder should be ignored
                        continue;
                    }

                    let new_dir_relative = format!("{dir}{file_name}/");
                    let new_read_dir = tokio::fs::read_dir(path_buf.join(&new_dir_relative)).await?;

                    stack.push((new_dir_relative.clone(), new_read_dir));

                    yield Ok((new_dir_relative, entry))
                } else {
                    let new_relative = format!("{dir}{file_name}");
                    yield Ok((new_relative, entry))
                }
            } else {
                log::trace!("read_end: {dir}");
                stack.pop();
                continue;
            };
        }
    }
}

async fn create_zip(
    backup_path: &Path,
    project_path: &Path,
    compression: async_zip::Compression,
    deflate_option: async_zip::DeflateOption,
) -> Result<(), RustError> {
    let mut file = tokio::fs::File::create(&backup_path).await?;
    let mut writer = async_zip::tokio::write::ZipFileWriter::with_tokio(&mut file);

    let mut stream = pin!(folder_stream(PathBuf::from(project_path)));

    while let Some((relative, entry)) = stream.try_next().await? {
        let mut file_type = entry.file_type().await?;
        if file_type.is_symlink() {
            file_type = match tokio::fs::metadata(entry.path()).await {
                Ok(metadata) => metadata.file_type(),
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
            };
        }
        if file_type.is_dir() {
            writer
                .write_entry_whole(
                    async_zip::ZipEntryBuilder::new(
                        relative.into(),
                        async_zip::Compression::Stored,
                    ),
                    b"",
                )
                .await?;
        } else {
            let file = tokio::fs::read(entry.path()).await?;
            writer
                .write_entry_whole(
                    async_zip::ZipEntryBuilder::new(relative.into(), compression)
                        .deflate_option(deflate_option),
                    file.as_ref(),
                )
                .await?;
        }
    }

    writer.close().await?;
    file.flush().await?;
    drop(file);
    Ok(())
}

struct RemoveOnDrop<'a>(&'a Path);

impl<'a> RemoveOnDrop<'a> {
    fn new(path: &'a Path) -> Self {
        RemoveOnDrop(path)
    }

    fn forget(self) {
        std::mem::forget(self);
    }
}

impl Drop for RemoveOnDrop<'_> {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.0);
    }
}

#[tauri::command]
#[specta::specta]
async fn project_create_backup(
    state: State<'_, Mutex<EnvironmentState>>,
    window: Window,
    channel: String,
    project_path: String,
) -> Result<AsyncCallResult<(), ()>, RustError> {
    async_command(channel, window, async {
        let (backup_dir, backup_format) = with_environment!(&state, |environment, config| {
            let backup_path = environment.project_backup_path();
            let backup_format = config.backup_format.to_ascii_lowercase();
            (backup_path.to_string(), backup_format)
        });

        With::<()>::continue_async(move |_| async move {
            let project_name = Path::new(&project_path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();

            let backup_name = format!(
                "{project_name}-{timestamp}",
                project_name = project_name,
                timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S"),
            );

            tokio::fs::create_dir_all(&backup_dir).await?;

            log::info!("backup project: {project_name} with {backup_format}");
            let timer = std::time::Instant::now();

            let backup_path;
            let remove_on_drop: RemoveOnDrop;
            match backup_format.as_str() {
                "default" | "zip-store" => {
                    backup_path = Path::new(&backup_dir)
                        .join(&backup_name)
                        .with_extension("zip");
                    remove_on_drop = RemoveOnDrop::new(&backup_path);
                    create_zip(
                        &backup_path,
                        project_path.as_ref(),
                        async_zip::Compression::Stored,
                        async_zip::DeflateOption::Normal,
                    )
                    .await?;
                }
                "zip-fast" => {
                    backup_path = Path::new(&backup_dir)
                        .join(&backup_name)
                        .with_extension("zip");
                    remove_on_drop = RemoveOnDrop::new(&backup_path);
                    create_zip(
                        &backup_path,
                        project_path.as_ref(),
                        async_zip::Compression::Deflate,
                        async_zip::DeflateOption::Other(1),
                    )
                    .await?;
                }
                "zip-best" => {
                    backup_path = Path::new(&backup_dir)
                        .join(&backup_name)
                        .with_extension("zip");
                    remove_on_drop = RemoveOnDrop::new(&backup_path);
                    create_zip(
                        &backup_path,
                        project_path.as_ref(),
                        async_zip::Compression::Deflate,
                        async_zip::DeflateOption::Other(9),
                    )
                    .await?;
                }
                backup_format => {
                    warn!("unknown backup format: {backup_format}, using zip-fast");

                    backup_path = Path::new(&backup_dir)
                        .join(&backup_name)
                        .with_extension("zip");

                    remove_on_drop = RemoveOnDrop::new(&backup_path);
                    create_zip(
                        &backup_path,
                        project_path.as_ref(),
                        async_zip::Compression::Deflate,
                        async_zip::DeflateOption::Other(1),
                    )
                    .await?;
                }
            }
            remove_on_drop.forget();

            log::info!("backup finished in {:?}", timer.elapsed());
            Ok(())
        })
    })
    .await
}

#[tauri::command]
#[specta::specta]
async fn util_open(path: String) -> Result<(), RustError> {
    open::that(path).map_err(RustError::unrecoverable)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
fn util_get_log_entries() -> Vec<LogEntry> {
    crate::logging::get_log_entries()
}

#[tauri::command]
#[specta::specta]
fn util_get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

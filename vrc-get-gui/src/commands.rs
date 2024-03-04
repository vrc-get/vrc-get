use log::{error, info};
use reqwest::Url;
use std::fmt::Display;
use std::io;
use std::num::Wrapping;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;
use specta::specta;
use tauri::api::dialog::blocking::FileDialogBuilder;
use tauri::async_runtime::Mutex;
use tauri::{generate_handler, Invoke, Runtime, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use crate::logging::LogEntry;
use vrc_get_vpm::environment::UserProject;
use vrc_get_vpm::unity_project::pending_project_changes::{
    ConflictInfo, PackageChange, RemoveReason,
};
use vrc_get_vpm::unity_project::{AddPackageOperation, PendingProjectChanges};
use vrc_get_vpm::version::Version;
use vrc_get_vpm::{
    PackageCollection, PackageInfo, PackageJson, ProjectType, VRCHAT_RECOMMENDED_2022_UNITY,
};

pub(crate) fn handlers<R: Runtime>() -> impl Fn(Invoke<R>) + Send + Sync + 'static {
    generate_handler![
        environment_projects,
        environment_add_project_with_picker,
        environment_packages,
        environment_repositories_info,
        environment_hide_repository,
        environment_show_repository,
        environment_set_hide_local_user_packages,
        project_details,
        project_install_package,
        project_upgrade_multiple_package,
        project_resolve,
        project_remove_package,
        project_apply_pending_changes,
        project_migrate_project_to_2022,
        project_finalize_migration_with_unity_2022,
        project_open_unity,
        util_open,
        util_get_log_entries,
        util_get_version,
    ]
}

#[cfg(debug_assertions)]
pub(crate) fn export_ts() {
    tauri_specta::ts::export_with_cfg(
        specta::collect_types![
            environment_projects,
            environment_add_project_with_picker,
            environment_packages,
            environment_repositories_info,
            environment_hide_repository,
            environment_show_repository,
            environment_set_hide_local_user_packages,
            project_details,
            project_install_package,
            project_upgrade_multiple_package,
            project_resolve,
            project_remove_package,
            project_apply_pending_changes,
            project_migrate_project_to_2022,
            project_finalize_migration_with_unity_2022::<tauri::Wry>,
            project_open_unity,
            util_open,
            util_get_log_entries,
            util_get_version,
        ]
        .unwrap(),
        specta::ts::ExportConfiguration::new().bigint(specta::ts::BigIntExportBehavior::Number),
        "lib/bindings.ts",
    )
    .unwrap();
}

pub(crate) fn new_env_state() -> impl Send + Sync + 'static {
    Mutex::new(EnvironmentState::new())
}

type Environment = vrc_get_vpm::Environment<reqwest::Client, vrc_get_vpm::io::DefaultEnvironmentIo>;
type UnityProject = vrc_get_vpm::UnityProject<vrc_get_vpm::io::DefaultProjectIo>;

async fn new_environment() -> io::Result<Environment> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("vrc-get-litedb/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("building client");
    let io = vrc_get_vpm::io::DefaultEnvironmentIo::new_default();
    Environment::load(Some(client), io).await
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
enum RustError {
    Unrecoverable(String),
}

impl RustError {
    fn unrecoverable<T: Display>(value: T) -> Self {
        error!("{value}");
        Self::Unrecoverable(value.to_string())
    }
}

impl From<io::Error> for RustError {
    fn from(value: io::Error) -> Self {
        RustError::unrecoverable(format!("io error: {value}"))
    }
}

unsafe impl Send for EnvironmentState {}

unsafe impl Sync for EnvironmentState {}

struct EnvironmentState {
    environment: EnvironmentHolder,
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
    environment_version: Wrapping<u32>,
}

impl EnvironmentHolder {
    fn new() -> Self {
        Self {
            environment: None,
            environment_version: Wrapping(0),
        }
    }

    async fn get_environment_mut(&mut self, inc_version: bool) -> io::Result<&mut Environment> {
        if let Some(ref mut environment) = self.environment {
            info!("reloading settings files");
            // reload settings files
            environment.reload().await?;
            if inc_version {
                self.environment_version += Wrapping(1);
            }
            Ok(environment)
        } else {
            self.environment = Some(new_environment().await?);
            self.environment_version += Wrapping(1);
            Ok(self.environment.as_mut().unwrap())
        }
    }
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
    fn new() -> Self {
        Self {
            environment: EnvironmentHolder::new(),
            packages: None,
            projects: Box::new([]),
            projects_version: Wrapping(0),
            changes_info: ChangesInfoHolder::new(),
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
    last_modified: u64,
    created_at: u64,
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
            last_modified: project.last_modified().as_millis_since_epoch(),
            created_at: project.crated_at().as_millis_since_epoch(),
            is_exists,
        }
    }
}

#[tauri::command]
#[specta::specta]
async fn environment_projects(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<Vec<TauriProject>, RustError> {
    let mut state = state.lock().await;
    let environment = state.environment.get_environment_mut(false).await?;

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
    InvalidFolderAsAProject,
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
        return Ok(TauriAddProjectWithPickerResult::InvalidFolderAsAProject);
    };

    let unity_project = load_project(project_path).await?;
    if !unity_project.is_valid().await {
        return Ok(TauriAddProjectWithPickerResult::InvalidFolderAsAProject);
    }

    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    let environment = env_state.environment.get_environment_mut(false).await?;

    environment.add_project(&unity_project).await?;
    environment.save().await?;

    Ok(TauriAddProjectWithPickerResult::Successful)
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
    is_yanked: bool,
}

impl TauriBasePackageInfo {
    fn new(package: &PackageJson) -> Self {
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
async fn environment_packages(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<Vec<TauriPackage>, RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    let environment = env_state.environment.get_environment_mut(true).await?;

    info!("loading package infos");
    environment.load_package_infos(true).await?;

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
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    let environment = env_state.environment.get_environment_mut(false).await?;

    Ok(TauriRepositoriesInfo {
        user_repositories: environment
            .get_user_repos()
            .iter()
            .map(|x| {
                let id = x.id().or(x.url().map(Url::as_str)).unwrap();
                TauriUserRepository {
                    id: id.to_string(),
                    display_name: x.name().unwrap_or(id).to_string(),
                }
            })
            .collect(),
        hidden_user_repositories: environment
            .gui_hidden_repositories()
            .map(Into::into)
            .collect(),
        hide_local_user_packages: environment.hide_local_user_packages(),
        show_prerelease_packages: environment.show_prerelease_packages(),
    })
}

#[tauri::command]
#[specta::specta]
async fn environment_hide_repository(
    state: State<'_, Mutex<EnvironmentState>>,
    repository: String,
) -> Result<(), RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    let environment = env_state.environment.get_environment_mut(false).await?;
    environment.add_gui_hidden_repositories(repository);
    environment.save().await?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn environment_show_repository(
    state: State<'_, Mutex<EnvironmentState>>,
    repository: String,
) -> Result<(), RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    let environment = env_state.environment.get_environment_mut(false).await?;
    environment.remove_gui_hidden_repositories(&repository);
    environment.save().await?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn environment_set_hide_local_user_packages(
    state: State<'_, Mutex<EnvironmentState>>,
    value: bool,
) -> Result<(), RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    let environment = env_state.environment.get_environment_mut(false).await?;
    environment.set_hide_local_user_packages(value);
    environment.save().await?;

    Ok(())
}

#[derive(Serialize, specta::Type)]
struct TauriProjectDetails {
    unity: Option<(u16, u8)>,
    unity_str: String,
    installed_packages: Vec<(String, TauriBasePackageInfo)>,
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
        unity_str: unity_project
            .unity_version()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".into()),
        installed_packages: unity_project
            .installed_packages()
            .map(|(k, p)| (k.to_string(), TauriBasePackageInfo::new(p)))
            .collect(),
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
                .map(|x| x.to_string_lossy().into_owned())
                .collect(),
            remove_legacy_folders: changes
                .remove_legacy_folders()
                .iter()
                .map(|x| x.to_string_lossy().into_owned())
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
    InstallNew(TauriBasePackageInfo),
    Remove(TauriRemoveReason),
}

impl TryFrom<&PackageChange<'_>> for TauriPackageChange {
    type Error = ();

    fn try_from(value: &PackageChange) -> Result<Self, ()> {
        Ok(match value {
            PackageChange::Install(install) => TauriPackageChange::InstallNew(
                TauriBasePackageInfo::new(install.install_package().ok_or(())?.package_json()),
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

#[tauri::command]
#[specta::specta]
async fn project_install_package(
    state: State<'_, Mutex<EnvironmentState>>,
    project_path: String,
    env_version: u32,
    package_index: usize,
) -> Result<TauriPendingProjectChanges, RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    if env_state.environment.environment_version != Wrapping(env_version) {
        return Err(RustError::Unrecoverable(
            "environment version mismatch".into(),
        ));
    }

    let environment = env_state.environment.get_environment_mut(false).await?;
    let packages = unsafe { &*env_state.packages.unwrap().as_mut() };
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

    let changes = match unity_project
        .add_package_request(
            environment,
            vec![installing_package],
            operation,
            allow_prerelease,
        )
        .await
    {
        Ok(request) => request,
        Err(e) => return Err(RustError::unrecoverable(e)),
    };

    Ok(env_state.changes_info.update(env_version, changes))
}

#[tauri::command]
#[specta::specta]
async fn project_upgrade_multiple_package(
    state: State<'_, Mutex<EnvironmentState>>,
    project_path: String,
    package_indices: Vec<(u32, usize)>,
) -> Result<TauriPendingProjectChanges, RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;

    let current_env_version = env_state.environment.environment_version;

    let environment = env_state.environment.get_environment_mut(false).await?;
    let packages = unsafe { &*env_state.packages.unwrap().as_mut() };
    let installing_packages = package_indices
        .iter()
        .map(|(env_version, index)| {
            if current_env_version != Wrapping(*env_version) {
                return Err(RustError::Unrecoverable(
                    "environment version mismatch".into(),
                ));
            }

            Ok(packages[*index])
        })
        .collect::<Result<_, _>>()?;

    let unity_project = load_project(project_path).await?;

    let operation = AddPackageOperation::UpgradeLocked;

    let allow_prerelease = environment.show_prerelease_packages();

    let changes = match unity_project
        .add_package_request(
            environment,
            installing_packages,
            operation,
            allow_prerelease,
        )
        .await
    {
        Ok(request) => request,
        Err(e) => return Err(RustError::unrecoverable(e)),
    };

    Ok(env_state
        .changes_info
        .update(current_env_version.0, changes))
}

#[tauri::command]
#[specta::specta]
async fn project_resolve(
    state: State<'_, Mutex<EnvironmentState>>,
    project_path: String,
) -> Result<TauriPendingProjectChanges, RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;

    let current_env_version = env_state.environment.environment_version;

    let environment = env_state.environment.get_environment_mut(false).await?;

    let unity_project = load_project(project_path).await?;

    let changes = match unity_project.resolve_request(environment).await {
        Ok(request) => request,
        Err(e) => return Err(RustError::unrecoverable(e)),
    };

    Ok(env_state
        .changes_info
        .update(current_env_version.0, changes))
}

#[tauri::command]
#[specta::specta]
async fn project_remove_package(
    state: State<'_, Mutex<EnvironmentState>>,
    project_path: String,
    name: String,
) -> Result<TauriPendingProjectChanges, RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    let env_version = env_state.environment.environment_version.0;

    let unity_project = load_project(project_path).await?;

    let changes = match unity_project.remove_request(&[&name]).await {
        Ok(request) => request,
        Err(e) => return Err(RustError::unrecoverable(e)),
    };

    Ok(env_state.changes_info.update(env_version, changes))
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

    let environment = env_state.environment.get_environment_mut(false).await?;

    let mut unity_project = load_project(project_path).await?;

    unity_project
        .apply_pending_changes(environment, changes.changes)
        .await?;

    unity_project.save().await?;
    update_project_last_modified(environment, unity_project.project_dir()).await;
    Ok(())
}

#[derive(Serialize, specta::Type)]
#[serde(tag = "type")]
enum TauriMigrateProjectTo2022Result {
    NoUnity2022Found,
    ConfirmNotExactlyRecommendedUnity2022 { found: String, recommended: String },
    MigrationInVpmFinished,
}

#[tauri::command]
#[specta::specta]
async fn project_migrate_project_to_2022(
    state: State<'_, Mutex<EnvironmentState>>,
    project_path: String,
    allow_mismatched_unity: bool,
) -> Result<TauriMigrateProjectTo2022Result, RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    let environment = env_state.environment.get_environment_mut(false).await?;

    let Some(found_unity) = environment.find_most_suitable_unity(VRCHAT_RECOMMENDED_2022_UNITY)?
    else {
        return Ok(TauriMigrateProjectTo2022Result::NoUnity2022Found);
    };

    if !allow_mismatched_unity && found_unity.version().unwrap() != VRCHAT_RECOMMENDED_2022_UNITY {
        return Ok(
            TauriMigrateProjectTo2022Result::ConfirmNotExactlyRecommendedUnity2022 {
                found: found_unity.version().unwrap().to_string(),
                recommended: VRCHAT_RECOMMENDED_2022_UNITY.to_string(),
            },
        );
    }

    let mut unity_project = load_project(project_path).await?;

    match unity_project.migrate_unity_2022(environment).await {
        Ok(()) => {}
        Err(e) => return Err(RustError::unrecoverable(e)),
    }

    unity_project.save().await?;
    update_project_last_modified(environment, unity_project.project_dir()).await;

    Ok(TauriMigrateProjectTo2022Result::MigrationInVpmFinished)
}

#[derive(Serialize, specta::Type)]
#[serde(tag = "type")]
enum TauriFinalizeMigrationWithUnity2022 {
    NoUnity2022Found,
    MigrationStarted { event_name: String },
}

// keep in sync with lib/migration-with-2022.ts
#[derive(Serialize, specta::Type, Clone)]
#[serde(tag = "type")]
enum TauriFinalizeMigrationWithUnity2022Event {
    OutputLine { line: String },
    ExistsWithNonZero { status: String },
    FinishedSuccessfully,
    Failed,
}

#[tauri::command]
#[specta::specta]
async fn project_finalize_migration_with_unity_2022<R: Runtime>(
    state: State<'_, Mutex<EnvironmentState>>,
    window: tauri::Window<R>,
    project_path: String,
) -> Result<TauriFinalizeMigrationWithUnity2022, RustError> {
    static MIGRATION_EVENT_PREFIX: &str = "migrateTo2022:";
    static MIGRATION_EVENT_COUNTER: AtomicU32 = AtomicU32::new(0);

    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    let environment = env_state.environment.get_environment_mut(false).await?;

    let Some(found_unity) = environment.find_most_suitable_unity(VRCHAT_RECOMMENDED_2022_UNITY)?
    else {
        return Ok(TauriFinalizeMigrationWithUnity2022::NoUnity2022Found);
    };
    environment.disconnect_litedb();

    let unity_project = load_project(project_path).await?;

    let mut child = Command::new(found_unity.path())
        .args([
            "-quit".as_ref(),
            "-batchmode".as_ref(),
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

    let id = MIGRATION_EVENT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let event_name = format!("{}{}", MIGRATION_EVENT_PREFIX, id);

    // stdout and stderr
    tokio::spawn(send_lines(
        child.stdout.take().unwrap(),
        window.clone(),
        event_name.clone(),
    ));
    tokio::spawn(send_lines(
        child.stderr.take().unwrap(),
        window.clone(),
        event_name.clone(),
    ));
    // process end
    tokio::spawn(wait_send_exit_status(child, window, event_name.clone()));

    async fn send_lines(
        stdout: impl tokio::io::AsyncRead + Unpin,
        window: tauri::Window<impl Runtime>,
        event_name: String,
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
                    let line = line.trim().to_string();
                    if let Err(e) = window.emit(
                        &event_name,
                        TauriFinalizeMigrationWithUnity2022Event::OutputLine { line },
                    ) {
                        match e {
                            tauri::Error::WebviewNotFound => break,
                            _ => error!("error sending stdout: {e}"),
                        }
                    }
                }
            }
        }
    }

    async fn wait_send_exit_status(
        mut child: Child,
        window: tauri::Window<impl Runtime>,
        event_name: String,
    ) {
        let event = match child.wait().await {
            Ok(status) => {
                if status.success() {
                    TauriFinalizeMigrationWithUnity2022Event::FinishedSuccessfully
                } else {
                    TauriFinalizeMigrationWithUnity2022Event::ExistsWithNonZero {
                        status: status.to_string(),
                    }
                }
            }
            Err(e) => {
                error!("error waiting for unity process: {e}");
                TauriFinalizeMigrationWithUnity2022Event::Failed
            }
        };
        window.emit(&event_name, event).unwrap();
    }

    Ok(TauriFinalizeMigrationWithUnity2022::MigrationStarted { event_name })
}

#[derive(Serialize, specta::Type)]
enum TauriOpenUnityResult {
    NoUnityVersionForTheProject,
    NoMatchingUnityFound,
    Success,
}

#[tauri::command]
#[specta::specta]
async fn project_open_unity(
    state: State<'_, Mutex<EnvironmentState>>,
    project_path: String,
) -> Result<TauriOpenUnityResult, RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    let environment = env_state.environment.get_environment_mut(false).await?;
    let unity_project = load_project(project_path).await?;

    let Some(project_unity) = unity_project.unity_version() else {
        return Ok(TauriOpenUnityResult::NoUnityVersionForTheProject);
    };

    for x in environment.get_unity_installations()? {
        if let Some(version) = x.version() {
            if version == project_unity {
                environment.disconnect_litedb();

                Command::new(x.path())
                    .args([
                        "-projectPath".as_ref(),
                        unity_project.project_dir().as_os_str(),
                    ])
                    .spawn()?;
                return Ok(TauriOpenUnityResult::Success);
            }
        }
    }

    environment.disconnect_litedb();

    Ok(TauriOpenUnityResult::NoMatchingUnityFound)
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

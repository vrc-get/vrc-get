use crate::commands::prelude::*;
use std::cmp::Reverse;

use crate::commands::async_command::{AsyncCallResult, AsyncCommandContext, With, async_command};
use crate::templates;
use crate::templates::{CreateProjectErr, ProjectTemplateInfo};
use crate::utils::{
    FileSystemTree, collect_notable_project_files_tree, default_project_path, trash_delete,
};
use futures::future::{join_all, try_join_all};
use futures::prelude::*;
use itertools::Itertools;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Component, Path, PathBuf, Prefix};
use std::sync::atomic::AtomicUsize;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, State, Window};
use tauri_plugin_dialog::DialogExt;
use vrc_get_vpm::ProjectType;
use vrc_get_vpm::environment::{
    PackageInstaller, RealProjectInformation, Settings, UserProject, VccDatabaseConnection,
};
use vrc_get_vpm::io::DefaultEnvironmentIo;
use vrc_get_vpm::version::UnityVersion;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TauriProject {
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
pub struct TauriUpdatedRealProjectInfo {
    // project information
    path: String,
    project_type: TauriProjectType,
    unity: String,
    unity_revision: Option<String>,
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
    fn new(project: &UserProject) -> Self {
        let is_exists = std::fs::metadata(project.path().unwrap())
            .map(|x| x.is_dir())
            .unwrap_or(false);
        Self {
            name: project.name().unwrap().to_string(),
            path: project.path().unwrap().to_string(),
            project_type: project.project_type().into(),
            unity: project
                .unity_version()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown".into()),
            unity_revision: project.unity_revision().map(|x| x.to_string()),
            last_modified: project
                .last_modified()
                .map(|x| x.as_unix_milliseconds())
                .unwrap_or(0),
            created_at: project
                .crated_at()
                .map(|x| x.as_unix_milliseconds())
                .unwrap_or(0),
            favorite: project.favorite(),
            is_exists,
        }
    }
}

impl TauriUpdatedRealProjectInfo {
    fn new(project: &RealProjectInformation) -> Self {
        Self {
            path: project.path().into(),
            project_type: project.project_type().into(),
            unity: project.unity_version().to_string(),
            unity_revision: project.unity_revision().map(Into::into),
        }
    }
}

async fn migrate_sanitize_projects(
    connection: &mut VccDatabaseConnection,
    io: &DefaultEnvironmentIo,
    settings: &Settings,
) -> io::Result<()> {
    info!("migrating projects from settings.json");
    // migrate from settings json
    connection.migrate(settings, io).await?;
    connection.dedup_projects();
    connection.normalize_path();
    Ok(())
}

fn sync_with_real_project_background(projects: &[UserProject], app: &AppHandle) {
    static LAST_UPDATE: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);
    // update after one minutes.
    let mut lock = LAST_UPDATE.lock().unwrap_or_else(|mut e| {
        **e.get_mut() = None;
        e.into_inner()
    });
    if lock
        .map(|x| x.elapsed() > std::time::Duration::from_secs(60))
        .unwrap_or(true)
    {
        *lock = Some(Instant::now());
        // start update thread
        log::info!("starting sync with real project...");
        tauri::async_runtime::spawn(sync_with_real_project(
            projects
                .iter()
                .map(|x| x.path().unwrap().to_string())
                .collect(),
            app.clone(),
        ));
    } else {
        log::info!("sync with real project skipped since last update is less than 1 minutes");
    }

    async fn sync_with_real_project(projects: Vec<String>, app: AppHandle) {
        app.emit("projects-update-in-progress", true).ok();

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        info!(
            "loading real project information of {} projects",
            projects.len()
        );

        let io = app.state::<DefaultEnvironmentIo>();

        let projects = join_all(projects.into_iter().map(async |project| {
            match RealProjectInformation::load_from_fs(&io, project.to_owned()).await {
                Ok(Some(project)) => {
                    app.emit(
                        "projects-updated",
                        TauriUpdatedRealProjectInfo::new(&project),
                    )
                    .ok();
                    Some(project)
                }
                Ok(None) => None,
                Err(err) => {
                    error!("Error updating project information: {}", err);
                    None
                }
            }
        }))
        .await;
        app.emit("projects-update-in-progress", false).ok();

        info!(
            "updating database real project information of {} projects",
            projects.len()
        );

        let mut connection = match VccDatabaseConnection::connect(io.inner()).await {
            Ok(connection) => connection,
            Err(e) => {
                error!("Error opening database: {}", e);
                return;
            }
        };
        connection.sync_with_real_projects_information(projects.into_iter().flatten().collect());
        match connection.save(io.inner()).await {
            Ok(()) => {}
            Err(e) => {
                error!("Error updating database: {}", e);
                return;
            }
        }

        info!("updated database based on real project information");
    }
}

#[tauri::command]
#[specta::specta]
pub async fn environment_projects(
    settings: State<'_, SettingsState>,
    io: State<'_, DefaultEnvironmentIo>,
    app: AppHandle,
) -> Result<Vec<TauriProject>, RustError> {
    let mut settings = settings.load_mut(io.inner()).await?;
    let mut connection = VccDatabaseConnection::connect(io.inner()).await?;

    migrate_sanitize_projects(&mut connection, io.inner(), &settings).await?;
    settings.load_from_db(&connection)?;
    connection.save(io.inner()).await?;
    settings.save().await?;

    info!("fetching projects");

    let mut projects = connection.get_projects();
    projects.retain(|x| x.path().is_some());

    sync_with_real_project_background(&projects, &app);

    let vec = projects.iter().map(TauriProject::new).collect::<Vec<_>>();

    Ok(vec)
}

#[derive(Serialize, specta::Type)]
pub enum TauriAddProjectWithPickerResult {
    NoFolderSelected,
    InvalidSelection,
    AlreadyAdded,
    Successful,
}

#[tauri::command]
#[specta::specta]
pub async fn environment_add_project_with_picker(
    settings: State<'_, SettingsState>,
    io: State<'_, DefaultEnvironmentIo>,
    window: Window,
) -> Result<TauriAddProjectWithPickerResult, RustError> {
    let Some(project_paths) = window
        .dialog()
        .file()
        .set_parent(&window)
        .blocking_pick_folders()
    else {
        return Ok(TauriAddProjectWithPickerResult::NoFolderSelected);
    };

    let Ok(project_paths) = project_paths
        .into_iter()
        .map(|x| x.into_path_buf().map_err(|_| ()))
        .map_ok(|x| x.into_os_string().into_string().map_err(|_| ()))
        .flatten_ok()
        .collect::<Result<Vec<_>, ()>>()
    else {
        return Ok(TauriAddProjectWithPickerResult::InvalidSelection);
    };

    let unity_projects = try_join_all(project_paths.into_iter().map(load_project)).await?;

    if stream::iter(unity_projects.iter())
        .any(async |p| !p.is_valid().await)
        .await
    {
        return Ok(TauriAddProjectWithPickerResult::InvalidSelection);
    }

    {
        let mut settings = settings.load_mut(io.inner()).await?;
        let mut connection = VccDatabaseConnection::connect(io.inner()).await?;
        migrate_sanitize_projects(&mut connection, io.inner(), &settings).await?;

        let projects = connection.get_projects();
        if (projects.iter().cartesian_product(unity_projects.iter()))
            .any(|(in_db, adding)| in_db.path().map(Path::new) == Some(adding.project_dir()))
        {
            return Ok(TauriAddProjectWithPickerResult::AlreadyAdded);
        }
        for unity_project in unity_projects {
            connection.add_project(&unity_project).await?;
        }
        connection.save(io.inner()).await?;
        settings.load_from_db(&connection)?;
        settings.save().await?;
    }

    Ok(TauriAddProjectWithPickerResult::Successful)
}

#[tauri::command]
#[specta::specta]
pub async fn environment_remove_project_by_path(
    settings: State<'_, SettingsState>,
    io: State<'_, DefaultEnvironmentIo>,
    project_path: String,
    directory: bool,
) -> Result<(), RustError> {
    let mut settings = settings.load_mut(io.inner()).await?;
    let mut connection = VccDatabaseConnection::connect(io.inner()).await?;
    migrate_sanitize_projects(&mut connection, io.inner(), &settings).await?;
    let Some(project) = connection.find_project(&project_path).unwrap() else {
        return Err(RustError::unrecoverable("project not found"));
    };
    connection.remove_project(&project);
    connection.save(io.inner()).await?;
    settings.load_from_db(&connection)?;
    settings.save().await?;

    if directory {
        let path = project.path().unwrap();
        info!("removing project directory: {path}");

        if let Err(err) = trash_delete(PathBuf::from(path)).await {
            error!("failed to remove project directory: {err}");
        } else {
            info!("removed project directory: {path}");
        }
    }

    Ok(())
}

#[derive(Serialize, specta::Type, Clone)]
pub struct TauriCopyProjectProgress {
    total: usize,
    proceed: usize,
    last_proceed: String,
}

#[tauri::command]
#[specta::specta]
pub async fn environment_copy_project_for_migration(
    window: Window,
    channel: String,
    source_path: String,
) -> Result<AsyncCallResult<TauriCopyProjectProgress, String>, RustError> {
    async fn create_folder(source_path: PathBuf) -> Option<PathBuf> {
        let folder = source_path.parent().unwrap();
        let name = source_path.file_name().unwrap();

        let name = name.to_str().unwrap();
        // first, try `-Migrated`
        let new_path = folder.join(format!("{name}-Migrated"));
        if let Ok(()) = tokio::fs::create_dir(&new_path).await {
            return Some(new_path);
        }

        for i in 1..100 {
            let new_path = folder.join(format!("{name}-Migrated-{i}"));
            if let Ok(()) = tokio::fs::create_dir(&new_path).await {
                return Some(new_path);
            }
        }

        None
    }

    copy_project(window, channel, source_path, create_folder).await
}

#[tauri::command]
#[specta::specta]
pub async fn environment_copy_project(
    window: Window,
    channel: String,
    source_path: String,
    new_path: String,
) -> Result<AsyncCallResult<TauriCopyProjectProgress, String>, RustError> {
    copy_project(window, channel, source_path, async move |_| {
        if let Ok(()) = tokio::fs::create_dir(&new_path).await {
            Some(PathBuf::from(new_path))
        } else {
            None
        }
    })
    .await
}

pub async fn copy_project<F, Fut>(
    window: Window,
    channel: String,
    source_path: String,
    create_folder: F,
) -> Result<AsyncCallResult<TauriCopyProjectProgress, String>, RustError>
where
    F: FnOnce(PathBuf) -> Fut + Send + Sync,
    Fut: Future<Output = Option<PathBuf>> + Send + Sync,
{
    async_command(channel, window, async {
        let source_path_str = source_path;
        let source_path = Path::new(&source_path_str);

        let Some(new_path) = create_folder(source_path.into()).await else {
            return Err(RustError::unrecoverable(
                "failed to create a new folder for migration",
            ));
        };
        let new_path_str = new_path.into_os_string().into_string().unwrap();

        With::<TauriCopyProjectProgress>::continue_async(move |ctx| async move {
            let source_path = Path::new(&source_path_str);
            let new_path = Path::new(&new_path_str);

            info!("copying project for migration: {source_path_str} -> {new_path_str}");

            let file_tree = collect_notable_project_files_tree(PathBuf::from(source_path)).await?;
            let total_files = file_tree.count_all();

            info!("collecting files for copy finished, total files: {total_files}");

            struct CopyFileContext<'a> {
                proceed: AtomicUsize,
                total_files: usize,
                new_path: &'a Path,
                ctx: &'a AsyncCommandContext<TauriCopyProjectProgress>,
            }

            impl CopyFileContext<'_> {
                fn on_finish(&self, entry: &FileSystemTree) {
                    let proceed = self
                        .proceed
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let last_proceed = entry.relative_path().to_string();

                    self.ctx
                        .emit(TauriCopyProjectProgress {
                            total: self.total_files,
                            proceed: proceed + 1,
                            last_proceed,
                        })
                        .ok();
                }

                async fn process(&self, entry: &FileSystemTree) -> io::Result<()> {
                    let new_entry = self.new_path.join(entry.relative_path());

                    if entry.is_dir() {
                        if let Err(e) = tokio::fs::create_dir(&new_entry).await {
                            if e.kind() != io::ErrorKind::AlreadyExists {
                                return Err(e);
                            }
                        }

                        try_join_all(entry.iter().map(|x| self.process(x))).await?;
                    } else {
                        tokio::fs::copy(entry.absolute_path(), new_entry).await?;

                        self.on_finish(entry);
                    }

                    Ok(())
                }
            }

            CopyFileContext {
                proceed: AtomicUsize::new(0),
                total_files,
                new_path,
                ctx: &ctx,
            }
            .process(&file_tree)
            .await?;

            info!("copied project for migration. adding to listing");

            let unity_project = load_project(new_path_str.clone()).await?;

            let settings = ctx.state::<SettingsState>();
            let io = ctx.state::<DefaultEnvironmentIo>();

            {
                let mut settings = settings.load_mut(io.inner()).await?;
                let mut connection = VccDatabaseConnection::connect(io.inner()).await?;
                migrate_sanitize_projects(&mut connection, io.inner(), &settings).await?;
                connection.add_project(&unity_project).await?;
                connection.save(io.inner()).await?;
                settings.load_from_db(&connection)?;
                settings.save().await?;
            }

            Ok(new_path_str)
        })
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_favorite_project(
    io: State<'_, DefaultEnvironmentIo>,
    project_path: String,
    favorite: bool,
) -> Result<(), RustError> {
    let mut connection = VccDatabaseConnection::connect(io.inner()).await?;
    let Some(mut project) = connection.find_project(&project_path).unwrap() else {
        return Err(RustError::unrecoverable("project not found"));
    };
    project.set_favorite(favorite);
    connection.update_project(&project);
    connection.save(io.inner()).await?;
    Ok(())
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum TauriProjectTemplate {
    Builtin { id: String, name: String },
    Custom { name: String },
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct TauriProjectTemplateInfo {
    pub display_name: String,
    pub id: String,
    pub unity_versions: Vec<String>,
    pub has_unitypackage: bool,
    pub source_path: Option<String>,
    pub available: bool,
}

impl From<&ProjectTemplateInfo> for TauriProjectTemplateInfo {
    fn from(info: &ProjectTemplateInfo) -> Self {
        Self {
            display_name: info.display_name.clone(),
            id: info.id.clone(),
            unity_versions: info
                .unity_versions
                .iter()
                .sorted_by_key(|&&x| Reverse(x))
                .map(|x| x.to_string())
                .unique()
                .collect(),
            has_unitypackage: info
                .alcom_template
                .as_ref()
                .map(|x| !x.unity_packages.is_empty())
                .unwrap_or(false),
            source_path: info
                .source_path
                .as_ref()
                .map(|x| x.to_string_lossy().into_owned()),
            available: info.available,
        }
    }
}

#[derive(Serialize, specta::Type)]
pub struct TauriProjectCreationInformation {
    templates: Vec<TauriProjectTemplateInfo>,
    templates_version: u32,
    default_path: String,
}

#[tauri::command]
#[specta::specta]
pub async fn environment_project_creation_information(
    settings: State<'_, SettingsState>,
    templates: State<'_, TemplatesState>,
    io: State<'_, DefaultEnvironmentIo>,
) -> Result<TauriProjectCreationInformation, RustError> {
    let unity_paths = {
        let connection = VccDatabaseConnection::connect(io.inner()).await?;

        connection
            .get_unity_installations()
            .iter()
            .filter_map(|unity| unity.version())
            .collect::<Vec<_>>()
    };

    let templates = templates.save(templates::load_resolve_all_templates(&io, &unity_paths).await?);

    let mut settings = settings.load_mut(io.inner()).await?;
    let default_path = default_project_path(&mut settings).to_string();
    settings.maybe_save().await?;

    Ok(TauriProjectCreationInformation {
        templates: templates.iter().map(Into::into).collect(),
        templates_version: templates.version(),
        default_path,
    })
}

#[derive(Serialize, specta::Type)]
pub enum TauriProjectDirCheckResult {
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
pub async fn environment_check_project_name(
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
pub enum TauriCreateProjectResult {
    AlreadyExists,
    TemplateNotFound,
    Successful,
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn environment_create_project(
    packages_state: State<'_, PackagesState>,
    settings: State<'_, SettingsState>,
    templates: State<'_, TemplatesState>,
    io: State<'_, DefaultEnvironmentIo>,
    http: State<'_, reqwest::Client>,
    base_path: String,
    project_name: String,
    template_id: String,
    template_version: u32,
    unity_version: String,
) -> Result<TauriCreateProjectResult, RustError> {
    let templates = templates
        .get_versioned(template_version)
        .ok_or_else(|| RustError::unrecoverable("Templates info version mismatch (bug)"))?;

    let unity_version = UnityVersion::parse(&unity_version)
        .ok_or_else(|| RustError::unrecoverable("Bad Unity Version (unparsable)"))?;

    let base_path = Path::new(&base_path);
    let base_path = {
        if !base_path.has_root() {
            let mut components = base_path.components().collect::<Vec<_>>();

            match (components.first(), components.get(1)) {
                (Some(Component::Prefix(_)), Some(Component::RootDir)) => {
                    // starts with 'C:/', good!
                }
                (Some(Component::Prefix(prefix)), _) => {
                    if matches!(prefix.kind(), Prefix::Disk(_)) {
                        // starts with 'C:yourpath', we should insert / after prefix
                        components.insert(1, Component::RootDir);
                    } else {
                        // starts with '\\?\', no problem
                    }
                }
                (Some(Component::RootDir), _) => {
                    // starts with '/', good!
                }
                (Some(_), _) => {
                    // starts with 'yourpath', insert '/'
                    components.insert(0, Component::RootDir);
                }
                _ => {}
            }

            components.iter().collect()
        } else {
            base_path.to_path_buf()
        }
    };
    let path = base_path.join(&project_name);

    // we split creating folder into two phases
    // because we want to fail if the project folder already exists.

    // create parent directory if not exists (unlikely to happen)
    super::super::create_dir_all_with_err(base_path).await?;

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

    let mut unity_project = match templates::create_project(
        &io,
        &templates,
        &template_id,
        &path,
        &project_name,
        unity_version,
    )
    .await
    {
        Ok(unity_project) => unity_project,
        Err(CreateProjectErr::Io(e)) => return Err(e.into()),
        Err(CreateProjectErr::NoSuchTemplate) => {
            return Ok(TauriCreateProjectResult::TemplateNotFound);
        }
    };

    let packages;
    {
        let mut settings = settings.load_mut(io.inner()).await?;
        packages = packages_state
            .load_fully(&settings, io.inner(), http.inner())
            .await?;

        // add the project to listing
        let mut connection = VccDatabaseConnection::connect(io.inner()).await?;
        migrate_sanitize_projects(&mut connection, io.inner(), &settings).await?;
        connection.add_project(&unity_project).await?;
        connection.save(io.inner()).await?;
        settings.load_from_db(&connection)?;
        settings.save().await?;
    }

    {
        let installer = PackageInstaller::new(io.inner(), Some(http.inner()));

        // finally, resolve the project folder
        let request = unity_project.resolve_request(packages.collection()).await?;
        unity_project
            .apply_pending_changes(&installer, request)
            .await?;
    }
    Ok(TauriCreateProjectResult::Successful)
}

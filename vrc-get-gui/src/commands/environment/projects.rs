use crate::commands::prelude::*;

use crate::commands::async_command::{async_command, AsyncCallResult, AsyncCommandContext, With};
use crate::utils::{collect_notable_project_files_tree, default_project_path, FileSystemTree};
use futures::future::try_join_all;
use futures::TryStreamExt;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::io;
use std::path::{Component, Path, PathBuf, Prefix};
use std::sync::atomic::AtomicUsize;
use tauri::{State, Window};
use tauri_plugin_dialog::DialogExt;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use vrc_get_vpm::environment::{PackageInstaller, Settings, UserProject, VccDatabaseConnection};
use vrc_get_vpm::io::{DefaultEnvironmentIo, DefaultProjectIo, DirEntry, EnvironmentIo, IoTrait};
use vrc_get_vpm::ProjectType;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TauriProject {
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

async fn migrate_sanitize_projects(
    connection: &mut VccDatabaseConnection,
    io: &DefaultEnvironmentIo,
    settings: &Settings,
) -> io::Result<()> {
    info!("migrating projects from settings.json");
    // migrate from settings json
    connection.migrate(settings, io).await?;
    connection.dedup_projects()?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_projects(
    settings: State<'_, SettingsState>,
    projects_state: State<'_, ProjectsState>,
    io: State<'_, DefaultEnvironmentIo>,
) -> Result<Vec<TauriProject>, RustError> {
    let mut settings = settings.load_mut(io.inner()).await?;
    let mut connection = VccDatabaseConnection::connect(io.inner()).await?;

    migrate_sanitize_projects(&mut connection, io.inner(), &settings).await?;
    info!("syncing information with real projects");
    connection.sync_with_real_projects(true, io.inner()).await?;
    settings.load_from_db(&connection)?;
    connection.save(io.inner()).await?;
    settings.save().await?;

    info!("fetching projects");

    let projects = connection.get_projects()?.into_boxed_slice();
    drop(connection);

    let stored = projects_state.set(projects).await;

    let vec = stored
        .data()
        .iter()
        .enumerate()
        .map(|(index, value)| TauriProject::new(stored.version(), index, value))
        .collect::<Vec<_>>();

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
    let Some(project_path) = window
        .dialog()
        .file()
        .set_parent(&window)
        .blocking_pick_folder()
        .map(|x| x.into_path_buf())
        .transpose()?
    else {
        return Ok(TauriAddProjectWithPickerResult::NoFolderSelected);
    };

    let Ok(project_path) = project_path.into_os_string().into_string() else {
        return Ok(TauriAddProjectWithPickerResult::InvalidSelection);
    };

    let unity_project = load_project(project_path.clone()).await?;
    if !unity_project.is_valid().await {
        return Ok(TauriAddProjectWithPickerResult::InvalidSelection);
    }

    {
        let mut settings = settings.load_mut(io.inner()).await?;
        let mut connection = VccDatabaseConnection::connect(io.inner()).await?;
        migrate_sanitize_projects(&mut connection, io.inner(), &settings).await?;

        let projects = connection.get_projects()?;
        if projects
            .iter()
            .any(|x| Path::new(x.path()) == Path::new(&project_path))
        {
            return Ok(TauriAddProjectWithPickerResult::AlreadyAdded);
        }
        connection.add_project(&unity_project).await?;
        connection.save(io.inner()).await?;
        settings.load_from_db(&connection)?;
        settings.save().await?;
    }

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
pub async fn environment_remove_project(
    projects_state: State<'_, ProjectsState>,
    settings: State<'_, SettingsState>,
    io: State<'_, DefaultEnvironmentIo>,
    list_version: u32,
    index: usize,
    directory: bool,
) -> Result<(), RustError> {
    let projects = projects_state.get().await;
    if list_version != projects.version() {
        return Err(RustError::unrecoverable("project list version mismatch"));
    }

    let Some(project) = projects.get(index) else {
        return Err(RustError::unrecoverable("project not found"));
    };

    let mut settings = settings.load_mut(io.inner()).await?;
    let mut connection = VccDatabaseConnection::connect(io.inner()).await?;
    migrate_sanitize_projects(&mut connection, io.inner(), &settings).await?;
    connection.remove_project(project)?;
    connection.save(io.inner()).await?;
    settings.load_from_db(&connection)?;
    settings.save().await?;

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
pub async fn environment_remove_project_by_path(
    settings: State<'_, SettingsState>,
    io: State<'_, DefaultEnvironmentIo>,
    path: String,
    directory: bool,
) -> Result<(), RustError> {
    {
        let mut settings = settings.load_mut(io.inner()).await?;
        let mut connection = VccDatabaseConnection::connect(io.inner()).await?;
        migrate_sanitize_projects(&mut connection, io.inner(), &settings).await?;

        let projects: Vec<UserProject> = connection.get_projects()?;

        if let Some(x) = projects.iter().find(|x| x.path() == path) {
            connection.remove_project(x)?;
            connection.save(io.inner()).await?;
            settings.load_from_db(&connection)?;
            settings.save().await?;
        } else {
            drop(settings);
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
    }
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

#[derive(Serialize, specta::Type, Clone)]
pub struct TauriCopyProjectForMigrationProgress {
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
) -> Result<AsyncCallResult<TauriCopyProjectForMigrationProgress, String>, RustError> {
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

    async_command(channel, window, async {
        let source_path_str = source_path;
        let source_path = Path::new(&source_path_str);
        let folder = source_path.parent().unwrap();
        let name = source_path.file_name().unwrap();

        let Some(new_path_str) = create_folder(folder, name).await else {
            return Err(RustError::unrecoverable(
                "failed to create a new folder for migration",
            ));
        };

        With::<TauriCopyProjectForMigrationProgress>::continue_async(move |ctx| async move {
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
                ctx: &'a AsyncCommandContext<TauriCopyProjectForMigrationProgress>,
            }

            impl<'a> CopyFileContext<'a> {
                fn on_finish(&self, entry: &FileSystemTree) {
                    let proceed = self
                        .proceed
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let last_proceed = entry.relative_path().to_string();

                    self.ctx
                        .emit(TauriCopyProjectForMigrationProgress {
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
    projects_state: State<'_, ProjectsState>,
    io: State<'_, DefaultEnvironmentIo>,
    list_version: u32,
    index: usize,
    favorite: bool,
) -> Result<(), RustError> {
    let mut projects = projects_state.get().await;
    if list_version != projects.version() {
        return Err(RustError::unrecoverable("project list version mismatch"));
    }
    let Some(project) = projects.get_mut(index) else {
        return Err(RustError::unrecoverable("project not found"));
    };

    project.set_favorite(favorite);

    let mut connection = VccDatabaseConnection::connect(io.inner()).await?;
    connection.update_project(project)?;
    connection.save(io.inner()).await?;

    Ok(())
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum TauriProjectTemplate {
    Builtin { id: String, name: String },
    Custom { name: String },
}

#[derive(Serialize, specta::Type)]
pub struct TauriProjectCreationInformation {
    templates: Vec<TauriProjectTemplate>,
    default_path: String,
}

async fn load_user_templates(io: &DefaultEnvironmentIo) -> io::Result<Vec<String>> {
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
pub async fn environment_project_creation_information(
    settings: State<'_, SettingsState>,
    io: State<'_, DefaultEnvironmentIo>,
) -> Result<TauriProjectCreationInformation, RustError> {
    {
        let mut templates = crate::templates::TEMPLATES
            .iter()
            .map(|&(id, name, _)| TauriProjectTemplate::Builtin {
                id: id.into(),
                name: name.into(),
            })
            .collect::<Vec<_>>();

        templates.extend(
            load_user_templates(&io)
                .await
                .ok()
                .into_iter()
                .flatten()
                .map(|name| TauriProjectTemplate::Custom { name }),
        );

        let mut settings = settings.load_mut(io.inner()).await?;
        let default_path = default_project_path(&mut settings).to_string();
        settings.maybe_save().await?;

        Ok(TauriProjectCreationInformation {
            templates,
            default_path,
        })
    }
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
pub async fn environment_create_project(
    packages_state: State<'_, PackagesState>,
    settings: State<'_, SettingsState>,
    io: State<'_, DefaultEnvironmentIo>,
    http: State<'_, reqwest::Client>,
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
            let template_path = io.resolve(format!("Templates/{name}").as_ref());
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
    let path = {
        let mut path;
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

            path = components.iter().collect();
        } else {
            path = base_path.to_path_buf();
        }
        path.push(&project_name);
        path
    };
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
        settings_file.sync_data().await?;
        drop(settings_file);
    }

    let packages;
    {
        let settings = settings.load(io.inner()).await?;
        packages = packages_state
            .load(&settings, io.inner(), http.inner())
            .await?;
    }

    {
        let installer = PackageInstaller::new(io.inner(), Some(http.inner()));

        let mut unity_project = load_project(path_str.into()).await?;

        // finally, resolve the project folder
        let request = unity_project.resolve_request(packages.collection()).await?;
        unity_project
            .apply_pending_changes(&installer, request)
            .await?;

        // add the project to listing
        let mut settings = settings.load_mut(io.inner()).await?;
        let mut connection = VccDatabaseConnection::connect(io.inner()).await?;
        migrate_sanitize_projects(&mut connection, io.inner(), &settings).await?;
        connection.add_project(&unity_project).await?;
        connection.save(io.inner()).await?;
        settings.load_from_db(&connection)?;
        settings.save().await?;
    }
    Ok(TauriCreateProjectResult::Successful)
}

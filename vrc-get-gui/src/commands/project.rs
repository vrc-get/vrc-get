use crate::commands::DEFAULT_UNITY_ARGUMENTS;
use crate::commands::async_command::*;
use crate::commands::prelude::*;
use crate::utils::{PathExt, collect_notable_project_files_tree, project_backup_path};
use log::{error, info, warn};
use serde::Serialize;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::str::FromStr;
use tauri::{AppHandle, State, Window};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use vrc_get_vpm::environment::{PackageInstaller, VccDatabaseConnection};
use vrc_get_vpm::io::DefaultEnvironmentIo;
use vrc_get_vpm::unity_project::pending_project_changes::{
    ConflictInfo, PackageChange, RemoveReason,
};
use vrc_get_vpm::unity_project::{AddPackageOperation, PendingProjectChanges};
use vrc_get_vpm::version::{StrictEqVersion, Version};

#[derive(Serialize, specta::Type)]
pub struct TauriProjectDetails {
    unity: (u16, u8),
    unity_str: String,
    unity_revision: Option<String>,
    installed_packages: Vec<(String, TauriBasePackageInfo)>,
    should_resolve: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn project_details(project_path: String) -> Result<TauriProjectDetails, RustError> {
    let unity_project = load_project(project_path).await?;

    Ok(TauriProjectDetails {
        unity: (
            unity_project.unity_version().major(),
            unity_project.unity_version().minor(),
        ),
        unity_str: unity_project.unity_version().to_string(),
        unity_revision: unity_project.unity_revision().map(|x| x.to_string()),
        installed_packages: unity_project
            .installed_packages()
            .map(|(k, p)| (k.to_string(), TauriBasePackageInfo::new(p)))
            .collect(),
        should_resolve: unity_project.should_resolve(),
    })
}

#[derive(Serialize, specta::Type)]
pub struct TauriPendingProjectChanges {
    changes_version: u32,
    package_changes: Vec<(String, TauriPackageChange)>,

    remove_legacy_files: Vec<String>,
    remove_legacy_folders: Vec<String>,

    conflicts: Vec<(String, TauriConflictInfo)>,
}

impl TauriPendingProjectChanges {
    pub fn new(version: u32, changes: &PendingProjectChanges) -> Self {
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
    unlocked_names: Vec<String>,
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
            unlocked_names: value
                .unlocked_names()
                .iter()
                .map(|x| x.to_string())
                .collect(),
        }
    }
}

macro_rules! changes {
    ($packages_ref: ident, $changes: ident, |$collection: pat_param, $packages: pat_param| $body: expr) => {{
        $changes
            .build_changes(
                &$packages_ref,
                |$collection, $packages| async { Ok($body) },
                TauriPendingProjectChanges::new,
            )
            .await
    }};
    ($packages_ref: ident, $changes: ident, |$collection: pat_param| $body: expr) => {{
        $changes
            .build_changes_no_list(
                &$packages_ref,
                |$collection| async { Ok($body) },
                TauriPendingProjectChanges::new,
            )
            .await
    }};
}

#[tauri::command]
#[specta::specta]
pub async fn project_install_packages(
    settings: State<'_, SettingsState>,
    packages: State<'_, PackagesState>,
    changes: State<'_, ChangesState>,
    io: State<'_, DefaultEnvironmentIo>,
    project_path: String,
    installs: Vec<(String, String)>,
) -> Result<TauriPendingProjectChanges, RustError> {
    let settings = settings.load(io.inner()).await?;
    let Some(packages) = packages.get() else {
        return Err(RustError::unrecoverable(
            "Internal Error: environment version mismatch",
        ));
    };
    let Some(installs) = installs
        .into_iter()
        .map(|(id, v)| Some((id, Version::from_str(&v).ok()?)))
        .collect::<Option<Vec<_>>>()
    else {
        return Err(RustError::unrecoverable("bad version file"));
    };

    changes!(packages, changes, |collection, packages| {
        let Some(installing_packages) = installs
            .iter()
            .map(|(id, version)| {
                packages
                    .iter()
                    .find(|&p| {
                        p.name() == id && StrictEqVersion(p.version()) == StrictEqVersion(version)
                    })
                    .copied()
            })
            .collect::<Option<Vec<_>>>()
        else {
            return Err(RustError::unrecoverable("some packages not found"));
        };

        let unity_project = load_project(project_path).await?;

        let allow_prerelease = settings.show_prerelease_packages();

        unity_project
            .add_package_request(
                collection,
                &installing_packages,
                AddPackageOperation::AutoDetected,
                allow_prerelease,
            )
            .await?
    })
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn project_reinstall_packages(
    app_handle: AppHandle,
    settings: State<'_, SettingsState>,
    packages: State<'_, PackagesState>,
    changes: State<'_, ChangesState>,
    io: State<'_, DefaultEnvironmentIo>,
    http: State<'_, reqwest::Client>,
    project_path: String,
    package_ids: Vec<String>,
) -> Result<TauriPendingProjectChanges, RustError> {
    let settings = settings.load(&io).await?;
    let packages = packages.load(&settings, &io, &http, app_handle).await?;

    changes!(packages, changes, |collection| {
        let unity_project = load_project(project_path).await?;

        let package_ids = package_ids.iter().map(|x| x.as_str()).collect::<Vec<_>>();

        unity_project
            .reinstall_request(collection, &package_ids)
            .await?
    })
}

#[tauri::command]
#[specta::specta]
pub async fn project_resolve(
    app_handle: AppHandle,
    settings: State<'_, SettingsState>,
    packages: State<'_, PackagesState>,
    changes: State<'_, ChangesState>,
    io: State<'_, DefaultEnvironmentIo>,
    http: State<'_, reqwest::Client>,
    project_path: String,
) -> Result<TauriPendingProjectChanges, RustError> {
    let settings = settings.load(&io).await?;
    let packages = packages.load(&settings, &io, &http, app_handle).await?;
    changes!(packages, changes, |collection| {
        let unity_project = load_project(project_path).await?;

        unity_project.resolve_request(collection).await?
    })
}

#[tauri::command]
#[specta::specta]
pub async fn project_remove_packages(
    changes_state: State<'_, ChangesState>,
    project_path: String,
    names: Vec<String>,
) -> Result<TauriPendingProjectChanges, RustError> {
    let unity_project = load_project(project_path).await?;

    let names = names.iter().map(|x| x.as_str()).collect::<Vec<_>>();

    let changes = unity_project.remove_request(&names).await?;

    Ok(changes_state.set(changes, TauriPendingProjectChanges::new))
}

#[tauri::command]
#[specta::specta]
pub async fn project_apply_pending_changes(
    changes: State<'_, ChangesState>,
    io: State<'_, DefaultEnvironmentIo>,
    http: State<'_, reqwest::Client>,
    project_path: String,
    changes_version: u32,
) -> Result<(), RustError> {
    let Some(mut changes) = changes.get_versioned(changes_version) else {
        return Err(RustError::unrecoverable("changes version mismatch"));
    };

    let changes = changes.take_changes();

    let installer = PackageInstaller::new(io.inner(), Some(http.inner()));

    let mut unity_project = load_project(project_path).await?;

    unity_project
        .apply_pending_changes(&installer, changes)
        .await?;

    update_project_last_modified(&io, unity_project.project_dir()).await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn project_clear_pending_changes(
    changes: State<'_, ChangesState>,
) -> Result<(), RustError> {
    changes.clear_cache();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn project_migrate_project_to_2022(
    app_handle: AppHandle,
    settings: State<'_, SettingsState>,
    packages: State<'_, PackagesState>,
    io: State<'_, DefaultEnvironmentIo>,
    http: State<'_, reqwest::Client>,
    project_path: String,
) -> Result<(), RustError> {
    {
        let settings = settings.load(io.inner()).await?;
        let packages = packages.load(&settings, &io, &http, app_handle).await?;
        let mut unity_project = load_project(project_path).await?;

        let installer = PackageInstaller::new(io.inner(), Some(http.inner()));

        unity_project
            .migrate_unity_2022(packages.collection(), &installer)
            .await?;

        update_project_last_modified(&io, unity_project.project_dir()).await;

        Ok(())
    }
}

#[derive(Serialize, specta::Type, Clone)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum TauriCallUnityForMigrationResult {
    ExistsWithNonZero { status: String },
    FinishedSuccessfully,
}

#[allow(dead_code)]
#[tauri::command]
#[specta::specta]
pub async fn project_call_unity_for_migration(
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
pub async fn project_migrate_project_to_vpm(
    app_handle: AppHandle,
    settings: State<'_, SettingsState>,
    packages: State<'_, PackagesState>,
    io: State<'_, DefaultEnvironmentIo>,
    http: State<'_, reqwest::Client>,
    project_path: String,
) -> Result<(), RustError> {
    let settings = settings.load(&io).await?;
    let packages = packages.load(&settings, &io, &http, app_handle).await?;

    let mut unity_project = load_project(project_path).await?;
    let installer = PackageInstaller::new(io.inner(), Some(http.inner()));

    unity_project
        .migrate_vpm(
            packages.collection(),
            &installer,
            settings.show_prerelease_packages(),
        )
        .await?;

    update_project_last_modified(&io, unity_project.project_dir()).await;

    Ok(())
}

fn is_unity_running(project_path: impl AsRef<Path>) -> bool {
    crate::os::is_locked(&project_path.as_ref().join("Temp/UnityLockFile")).unwrap_or(false)
}

#[tauri::command]
#[specta::specta]
pub async fn project_open_unity(
    config: State<'_, GuiConfigState>,
    io: State<'_, DefaultEnvironmentIo>,
    project_path: String,
    unity_path: String,
) -> Result<bool, RustError> {
    if is_unity_running(&project_path) {
        // it looks unity is running. returning false
        return Ok(false);
    }

    let mut custom_args: Option<Vec<String>> = None;

    {
        let mut connection = VccDatabaseConnection::connect(io.inner()).await?;
        if let Some(project) = connection.find_project(project_path.as_ref())? {
            custom_args = project
                .custom_unity_args()
                .map(|x| Vec::from_iter(x.iter().map(ToOwned::to_owned)));
        }
        connection.update_project_last_modified(project_path.as_ref())?;
        connection.save(io.inner()).await?;
    }

    let unity_args = custom_args.or_else(|| config.get().default_unity_arguments.clone());
    tokio::spawn(async move {
        let mut args = vec!["-projectPath".as_ref(), OsStr::new(project_path.as_str())];

        if let Some(unity_args) = &unity_args {
            args.extend(unity_args.iter().map(OsStr::new));
        } else {
            args.extend(DEFAULT_UNITY_ARGUMENTS.iter().map(OsStr::new));
        }

        if let Err(e) = crate::os::start_command("Unity".as_ref(), unity_path.as_ref(), &args).await
        {
            log::error!("Launching Unity: {e}");
        }
    });

    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub fn project_is_unity_launching(project_path: String) -> bool {
    is_unity_running(project_path)
}

#[tauri::command]
#[specta::specta]
pub fn project_bring_unity_to_foreground(project_path: String) -> Result<bool, RustError> {
    bring_unity_to_foreground(project_path)
}

fn bring_unity_to_foreground(project_path: String) -> Result<bool, RustError> {
    crate::os::bring_unity_to_foreground(std::path::Path::new(&project_path))
        .map_err(RustError::unrecoverable)
}

async fn create_backup_zip(
    backup_path: &Path,
    project_path: &Path,
    compression: async_zip::Compression,
    deflate_option: async_zip::DeflateOption,
    exclude_vpm: bool,
    ctx: AsyncCommandContext<TauriCreateBackupProgress>,
) -> Result<(), RustError> {
    let mut file = tokio::fs::File::create_new(&backup_path).await?;
    let mut writer = async_zip::tokio::write::ZipFileWriter::with_tokio(&mut file);

    info!("Collecting files to backup {}...", project_path.display());

    let start = std::time::Instant::now();
    let file_tree =
        collect_notable_project_files_tree(PathBuf::from(project_path), exclude_vpm, true).await?;

    let total_files = file_tree.count_all();

    info!(
        "Collecting files took {}, starting creating archive with {total_files} files...",
        start.elapsed().as_secs_f64()
    );

    let _ = ctx.emit(TauriCreateBackupProgress {
        total: total_files,
        proceed: 0,
        last_proceed: "Collecting files".to_string(),
    });

    for (proceed, entry) in file_tree.recursive().enumerate() {
        if entry.is_dir() {
            writer
                .write_entry_whole(
                    async_zip::ZipEntryBuilder::new(
                        entry.relative_path().into(),
                        async_zip::Compression::Stored,
                    ),
                    b"",
                )
                .await?;
        } else {
            let file = tokio::fs::read(entry.absolute_path()).await?;
            writer
                .write_entry_whole(
                    async_zip::ZipEntryBuilder::new(entry.relative_path().into(), compression)
                        .deflate_option(deflate_option),
                    file.as_ref(),
                )
                .await?;
        }

        let _ = ctx.emit(TauriCreateBackupProgress {
            total: total_files,
            proceed: proceed + 1,
            last_proceed: entry.relative_path().to_string(),
        });
    }

    writer.close().await?;
    file.flush().await?;
    file.sync_data().await?;
    drop(file);

    info!(
        "Creating backup archive for {} finished!",
        project_path.display()
    );

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

#[derive(Serialize, specta::Type, Clone)]
pub struct TauriCreateBackupProgress {
    total: usize,
    proceed: usize,
    last_proceed: String,
}

#[tauri::command]
#[specta::specta]
pub async fn project_create_backup(
    config: State<'_, GuiConfigState>,
    settings: State<'_, SettingsState>,
    io: State<'_, DefaultEnvironmentIo>,
    window: Window,
    channel: String,
    project_path: String,
) -> Result<AsyncCallResult<TauriCreateBackupProgress, ()>, RustError> {
    async_command(channel, window, async {
        let backup_format = config.get().backup_format.to_ascii_lowercase();
        let exclude_vpm = config.get().exclude_vpm_packages_from_backup;

        let mut settings = settings.load_mut(io.inner()).await?;
        let backup_dir = project_backup_path(&mut settings).to_string();
        settings.maybe_save().await?;

        With::<TauriCreateBackupProgress>::continue_async(move |ctx| async move {
            let project_name = Path::new(&project_path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();

            let backup_name = format!(
                "{project_name}-{timestamp}",
                project_name = project_name,
                timestamp = chrono::Local::now().format("%Y-%m-%dT%H-%M-%S"),
            );

            super::create_dir_all_with_err(&backup_dir).await?;

            log::info!("backup project: {project_name} with {backup_format}");
            let timer = std::time::Instant::now();

            let backup_path;
            let remove_on_drop: RemoveOnDrop;
            match backup_format.as_str() {
                "default" | "zip-store" => {
                    backup_path = Path::new(&backup_dir)
                        .join(&backup_name)
                        .with_added_extension("zip");
                    remove_on_drop = RemoveOnDrop::new(&backup_path);
                    create_backup_zip(
                        &backup_path,
                        project_path.as_ref(),
                        async_zip::Compression::Stored,
                        async_zip::DeflateOption::Normal,
                        exclude_vpm,
                        ctx,
                    )
                    .await?;
                }
                "zip-fast" => {
                    backup_path = Path::new(&backup_dir)
                        .join(&backup_name)
                        .with_added_extension("zip");
                    remove_on_drop = RemoveOnDrop::new(&backup_path);
                    create_backup_zip(
                        &backup_path,
                        project_path.as_ref(),
                        async_zip::Compression::Deflate,
                        async_zip::DeflateOption::Other(1),
                        exclude_vpm,
                        ctx,
                    )
                    .await?;
                }
                "zip-best" => {
                    backup_path = Path::new(&backup_dir)
                        .join(&backup_name)
                        .with_added_extension("zip");
                    remove_on_drop = RemoveOnDrop::new(&backup_path);
                    create_backup_zip(
                        &backup_path,
                        project_path.as_ref(),
                        async_zip::Compression::Deflate,
                        async_zip::DeflateOption::Other(9),
                        exclude_vpm,
                        ctx,
                    )
                    .await?;
                }
                backup_format => {
                    warn!("unknown backup format: {backup_format}, using zip-fast");

                    backup_path = Path::new(&backup_dir)
                        .join(&backup_name)
                        .with_added_extension("zip");

                    remove_on_drop = RemoveOnDrop::new(&backup_path);
                    create_backup_zip(
                        &backup_path,
                        project_path.as_ref(),
                        async_zip::Compression::Deflate,
                        async_zip::DeflateOption::Other(1),
                        exclude_vpm,
                        ctx,
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
pub async fn project_get_custom_unity_args(
    io: State<'_, DefaultEnvironmentIo>,
    project_path: String,
) -> Result<Option<Vec<String>>, RustError> {
    let connection = VccDatabaseConnection::connect(io.inner()).await?;
    if let Some(project) = connection.find_project(project_path.as_ref())? {
        Ok(project
            .custom_unity_args()
            .map(|x| x.iter().map(ToOwned::to_owned).collect()))
    } else {
        Ok(None)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn project_set_custom_unity_args(
    io: State<'_, DefaultEnvironmentIo>,
    project_path: String,
    args: Option<Vec<String>>,
) -> Result<bool, RustError> {
    let mut connection = VccDatabaseConnection::connect(io.inner()).await?;
    if let Some(mut project) = connection.find_project(project_path.as_ref())? {
        if let Some(args) = args {
            project.set_custom_unity_args(args);
        } else {
            project.clear_custom_unity_args();
        }
        connection.update_project(&project);
        connection.save(io.inner()).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn project_get_unity_path(
    io: State<'_, DefaultEnvironmentIo>,
    project_path: String,
) -> Result<Option<String>, RustError> {
    let connection = VccDatabaseConnection::connect(io.inner()).await?;
    if let Some(project) = connection.find_project(project_path.as_ref())? {
        Ok(project.unity_path().map(ToOwned::to_owned))
    } else {
        Ok(None)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn project_set_unity_path(
    io: State<'_, DefaultEnvironmentIo>,
    project_path: String,
    unity_path: Option<String>,
) -> Result<bool, RustError> {
    let mut connection = VccDatabaseConnection::connect(io.inner()).await?;
    if let Some(mut project) = connection.find_project(project_path.as_ref())? {
        if let Some(unity_path) = unity_path {
            project.set_unity_path(unity_path);
        } else {
            project.clear_unity_path();
        }
        connection.update_project(&project);
        connection.save(io.inner()).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

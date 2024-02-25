use serde::Serialize;
use specta::specta;
use std::io;
use std::num::Wrapping;
use std::path::PathBuf;
use std::ptr::NonNull;
use tauri::async_runtime::Mutex;
use tauri::{generate_handler, Invoke, Runtime, State};
use vrc_get_vpm::environment::UserProject;
use vrc_get_vpm::io::DefaultProjectIo;
use vrc_get_vpm::version::Version;
use vrc_get_vpm::{PackageCollection, PackageInfo, PackageJson, ProjectType, UnityProject};

pub(crate) fn handlers<R: Runtime>() -> impl Fn(Invoke<R>) + Send + Sync + 'static {
    generate_handler![environment_projects, environment_packages, project_details]
}

#[cfg(debug_assertions)]
pub(crate) fn export_ts() {
    tauri_specta::ts::export_with_cfg(
        specta::collect_types![environment_projects, environment_packages, project_details]
            .unwrap(),
        specta::ts::ExportConfiguration::new().bigint(specta::ts::BigIntExportBehavior::Number),
        "web/lib/bindings.ts",
    )
    .unwrap();
}

pub(crate) fn new_env_state() -> impl Send + Sync + 'static {
    Mutex::new(EnvironmentState::new())
}

type Environment = vrc_get_vpm::Environment<reqwest::Client, vrc_get_vpm::io::DefaultEnvironmentIo>;

async fn new_environment() -> io::Result<Environment> {
    let client = reqwest::Client::new();
    let io = vrc_get_vpm::io::DefaultEnvironmentIo::new_default();
    Environment::load(Some(client), io).await
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[specta(export)]
enum RustError {
    Unrecoverable(String),
}

impl From<io::Error> for RustError {
    fn from(value: io::Error) -> Self {
        RustError::Unrecoverable(format!("io error: {value}"))
    }
}

unsafe impl Send for EnvironmentState {}
unsafe impl Sync for EnvironmentState {}

struct EnvironmentState {
    environment: EnvironmentHolder,
    packages: Option<NonNull<[PackageInfo<'static>]>>, // null or reference to
    projects: Box<[UserProject]>,
    projects_version: Wrapping<u32>,
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
            println!("reloading settings files");
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

impl EnvironmentState {
    fn new() -> Self {
        Self {
            environment: EnvironmentHolder::new(),
            packages: None,
            projects: Box::new([]),
            projects_version: Wrapping(0),
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

    println!("migrating projects from settings.json");
    // migrate from settings json
    environment.migrate_from_settings_json().await?;
    environment.save().await?;

    println!("fetching projects");

    state.projects = environment.get_projects()?.into_boxed_slice();
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
    version: TauriVersion,
    unity: Option<(u16, u8)>,
    is_yanked: bool,
}

impl TauriBasePackageInfo {
    fn new(package: &PackageJson) -> Self {
        Self {
            name: package.name().to_string(),
            display_name: package.display_name().map(|v| v.to_string()),
            version: package.version().into(),
            unity: package.unity().map(|v| (v.major(), v.minor())),
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

    println!("loading package infos");
    environment.load_package_infos(true).await?;

    let packages = environment
        .get_all_packages()
        .collect::<Vec<_>>()
        .into_boxed_slice();
    if let Some(ptr) = env_state.packages {
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
struct TauriProjectDetails {
    unity: Option<(u16, u8)>,
    unity_str: String,
    installed_packages: Vec<(String, TauriBasePackageInfo)>,
}

#[tauri::command]
#[specta::specta]
async fn project_details(project_path: String) -> Result<TauriProjectDetails, RustError> {
    let unity_project =
        UnityProject::load(DefaultProjectIo::new(PathBuf::from(project_path).into())).await?;

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

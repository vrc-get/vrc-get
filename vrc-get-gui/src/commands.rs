use serde::Serialize;
use specta::specta;
use std::io;
use tauri::async_runtime::Mutex;
use tauri::{generate_handler, Invoke, Runtime, State};
use vrc_get_vpm::environment::UserProject;
use vrc_get_vpm::ProjectType;

pub(crate) fn handlers<R: Runtime>() -> impl Fn(Invoke<R>) + Send + Sync + 'static {
    generate_handler![environment_projects]
}

#[cfg(debug_assertions)]
pub(crate) fn export_ts() {
    tauri_specta::ts::export_with_cfg(
        specta::collect_types![environment_projects].unwrap(),
        specta::ts::ExportConfiguration::new().bigint(specta::ts::BigIntExportBehavior::Number),
        "web/lib/generated/bindings.ts",
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

struct EnvironmentState {
    environment: Option<Environment>,
    projects: Box<[UserProject]>,
    projects_version: u32,
}

impl EnvironmentState {
    fn new() -> Self {
        Self {
            environment: None,
            projects: Box::new([]),
            projects_version: 0,
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
    let environment = match state.environment.as_mut() {
        Some(s) => s,
        None => {
            state.environment = Some(new_environment().await?);
            state.environment.as_mut().unwrap()
        }
    };

    state.projects = environment.get_projects()?.into_boxed_slice();
    state.projects_version = state.projects_version.wrapping_add(1);

    let vec = state
        .projects
        .iter()
        .enumerate()
        .map(|(index, value)| TauriProject::new(state.projects_version, index, value))
        .collect::<Vec<_>>();

    Ok(vec)
}

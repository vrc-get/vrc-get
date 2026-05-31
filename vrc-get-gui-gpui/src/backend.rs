use std::path::PathBuf;

use vrc_get_vpm::ProjectType;
use vrc_get_vpm::environment::{VccDatabaseConnection, UserProject};
use vrc_get_vpm::io::DefaultEnvironmentIo;

#[derive(Clone, Debug)]
pub struct ProjectRow {
    pub name: String,
    pub path: String,
    pub project_type: String,
    pub unity: String,
    pub favorite: bool,
    pub last_modified_ms: i64,
}

impl ProjectRow {
    fn from_user_project(p: &UserProject) -> Option<Self> {
        let path = p.path()?.to_owned();
        let name = PathBuf::from(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&path)
            .to_owned();
        let project_type = project_type_label(p.project_type());
        let unity = p
            .unity_version()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_owned());
        let favorite = p.favorite();
        let last_modified_ms = p
            .last_modified()
            .map(|d| d.as_unix_milliseconds())
            .unwrap_or(0);

        Some(ProjectRow {
            name,
            path,
            project_type,
            unity,
            favorite,
            last_modified_ms,
        })
    }
}

fn project_type_label(t: ProjectType) -> String {
    match t {
        ProjectType::Unknown => "Unknown",
        ProjectType::LegacySdk2 => "Legacy SDK2",
        ProjectType::LegacyWorlds => "Legacy Worlds",
        ProjectType::LegacyAvatars => "Legacy Avatars",
        ProjectType::UpmWorlds => "UPM Worlds",
        ProjectType::UpmAvatars => "UPM Avatars",
        ProjectType::UpmStarter => "UPM Starter",
        ProjectType::Worlds => "Worlds",
        ProjectType::Avatars => "Avatars",
        ProjectType::VpmStarter => "VPM Starter",
    }
    .to_owned()
}

/// Load all projects from the VCC database.  Intended to be called from a
/// Tokio context (via `TokioBridge::call`).
pub async fn load_projects() -> anyhow::Result<Vec<ProjectRow>> {
    let io = DefaultEnvironmentIo::new_default();
    let connection = VccDatabaseConnection::connect(&io).await?;

    let mut projects = connection.get_projects();
    projects.retain(|p| p.path().is_some());

    let rows = projects
        .iter()
        .filter_map(ProjectRow::from_user_project)
        .collect();

    Ok(rows)
}

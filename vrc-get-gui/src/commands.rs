use std::fmt::Display;
use std::io;
use std::path::{Path, PathBuf};

use log::error;
use serde::Serialize;
use specta::specta;
use tauri::{generate_handler, Invoke};

pub use start::startup;
pub use state::new_env_state;
use vrc_get_vpm::io::{DefaultEnvironmentIo, DefaultProjectIo};
use vrc_get_vpm::version::Version;
use vrc_get_vpm::PackageManifest;

#[macro_use]
mod state;
mod async_command;
mod environment;
mod project;
mod start;
mod util;

mod prelude {
    pub use super::state::{EnvironmentState, UpdateRepositoryMode};
    pub(super) use super::{
        load_project, update_project_last_modified, Environment, RustError, TauriBasePackageInfo,
        UnityProject,
    };
}

pub type Environment = vrc_get_vpm::Environment<reqwest::Client, DefaultEnvironmentIo>;
pub type UnityProject = vrc_get_vpm::UnityProject<DefaultProjectIo>;

pub(crate) fn handlers() -> impl Fn(Invoke) + Send + Sync + 'static {
    generate_handler![
        environment::config::environment_language,
        environment::config::environment_set_language,
        environment::config::environment_theme,
        environment::config::environment_set_theme,
        environment::config::environment_get_project_sorting,
        environment::config::environment_set_project_sorting,
        environment::projects::environment_projects,
        environment::projects::environment_add_project_with_picker,
        environment::projects::environment_remove_project,
        environment::projects::environment_remove_project_by_path,
        environment::projects::environment_copy_project_for_migration,
        environment::projects::environment_set_favorite_project,
        environment::projects::environment_project_creation_information,
        environment::projects::environment_check_project_name,
        environment::projects::environment_create_project,
        environment::packages::environment_refetch_packages,
        environment::packages::environment_packages,
        environment::packages::environment_repositories_info,
        environment::packages::environment_hide_repository,
        environment::packages::environment_show_repository,
        environment::packages::environment_set_hide_local_user_packages,
        environment::packages::environment_download_repository,
        environment::packages::environment_add_repository,
        environment::packages::environment_remove_repository,
        environment::settings::environment_unity_versions,
        environment::settings::environment_get_settings,
        environment::settings::environment_pick_unity_hub,
        environment::settings::environment_pick_unity,
        environment::settings::environment_pick_project_default_path,
        environment::settings::environment_pick_project_backup_path,
        environment::settings::environment_set_show_prerelease_packages,
        environment::settings::environment_set_backup_format,
        project::project_details,
        project::project_install_package,
        project::project_install_multiple_package,
        project::project_upgrade_multiple_package,
        project::project_resolve,
        project::project_remove_packages,
        project::project_apply_pending_changes,
        project::project_migrate_project_to_2022,
        project::project_call_unity_for_migration,
        project::project_migrate_project_to_vpm,
        project::project_open_unity,
        project::project_is_unity_launching,
        project::project_create_backup,
        project::project_get_custom_unity_args,
        project::project_set_custom_unity_args,
        project::project_get_unity_path,
        project::project_set_unity_path,
        util::util_open,
        util::util_get_log_entries,
        util::util_get_version,
        util::util_check_for_update,
        util::util_install_and_upgrade,
        crate::deep_link_support::deep_link_has_add_repository,
        crate::deep_link_support::deep_link_take_add_repository,
        crate::deep_link_support::deep_link_install_vcc,
    ]
}

#[cfg(debug_assertions)]
pub(crate) fn export_ts() {
    tauri_specta::ts::export_with_cfg(
        specta::collect_types![
            environment::config::environment_language,
            environment::config::environment_set_language,
            environment::config::environment_theme,
            environment::config::environment_set_theme,
            environment::config::environment_get_project_sorting,
            environment::config::environment_set_project_sorting,
            environment::projects::environment_projects,
            environment::projects::environment_add_project_with_picker,
            environment::projects::environment_remove_project,
            environment::projects::environment_remove_project_by_path,
            environment::projects::environment_copy_project_for_migration,
            environment::projects::environment_set_favorite_project,
            environment::projects::environment_project_creation_information,
            environment::projects::environment_check_project_name,
            environment::projects::environment_create_project,
            environment::packages::environment_refetch_packages,
            environment::packages::environment_packages,
            environment::packages::environment_repositories_info,
            environment::packages::environment_hide_repository,
            environment::packages::environment_show_repository,
            environment::packages::environment_set_hide_local_user_packages,
            environment::packages::environment_download_repository,
            environment::packages::environment_add_repository,
            environment::packages::environment_remove_repository,
            environment::settings::environment_unity_versions,
            environment::settings::environment_get_settings,
            environment::settings::environment_pick_unity_hub,
            environment::settings::environment_pick_unity,
            environment::settings::environment_pick_project_default_path,
            environment::settings::environment_pick_project_backup_path,
            environment::settings::environment_set_show_prerelease_packages,
            environment::settings::environment_set_backup_format,
            project::project_details,
            project::project_install_package,
            project::project_install_multiple_package,
            project::project_upgrade_multiple_package,
            project::project_resolve,
            project::project_remove_packages,
            project::project_apply_pending_changes,
            project::project_migrate_project_to_2022,
            project::project_call_unity_for_migration,
            project::project_migrate_project_to_vpm,
            project::project_open_unity,
            project::project_is_unity_launching,
            project::project_create_backup,
            project::project_get_custom_unity_args,
            project::project_set_custom_unity_args,
            project::project_get_unity_path,
            project::project_set_unity_path,
            util::util_open,
            util::util_get_log_entries,
            util::util_get_version,
            util::util_check_for_update,
            util::util_install_and_upgrade,
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
    description: Option<String>,
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
            description: package.description().map(|v| v.to_string()),
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

async fn load_project(project_path: String) -> Result<UnityProject, RustError> {
    Ok(UnityProject::load(DefaultProjectIo::new(PathBuf::from(project_path).into())).await?)
}

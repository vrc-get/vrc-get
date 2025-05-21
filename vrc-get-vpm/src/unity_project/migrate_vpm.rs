use futures::prelude::*;
use log::{debug, info};
use std::collections::HashSet;

use crate::io::IoTrait;
use crate::unity_project::{AddPackageErr, AddPackageOperation};
use crate::{PackageCollection, UnityProject, VersionSelector};
use crate::{PackageInstaller, ProjectType, io};

#[non_exhaustive]
#[derive(Debug)]
pub enum MigrateVpmError {
    ProjectTypeMismatch(ProjectType),
    UnityVersionMismatch,
    VpmPackageNotFound(&'static str),
    AddPackageErr(AddPackageErr),
    Io(io::Error),
}

impl std::error::Error for MigrateVpmError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MigrateVpmError::AddPackageErr(err) => Some(err),
            MigrateVpmError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl std::fmt::Display for MigrateVpmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrateVpmError::ProjectTypeMismatch(either) => {
                write!(f, "Project type is {:?}", either)
            }
            MigrateVpmError::UnityVersionMismatch => write!(f, "Unity version is not 2019.x"),
            MigrateVpmError::VpmPackageNotFound(name) => {
                write!(f, "VPM package {} not found", name)
            }
            MigrateVpmError::AddPackageErr(err) => write!(f, "{}", err),
            MigrateVpmError::Io(err) => write!(f, "{}", err),
        }
    }
}

impl From<AddPackageErr> for MigrateVpmError {
    fn from(err: AddPackageErr) -> Self {
        MigrateVpmError::AddPackageErr(err)
    }
}

impl From<io::Error> for MigrateVpmError {
    fn from(err: io::Error) -> Self {
        MigrateVpmError::Io(err)
    }
}

type Result<T = (), E = MigrateVpmError> = std::result::Result<T, E>;

impl UnityProject {
    pub async fn migrate_vpm(
        &mut self,
        collection: &impl PackageCollection,
        installer: &impl PackageInstaller,
        include_prerelease: bool,
    ) -> Result {
        migrate_vpm(self, collection, installer, include_prerelease).await
    }
}

async fn migrate_vpm(
    project: &mut UnityProject,
    collection: &impl PackageCollection,
    installer: &impl PackageInstaller,
    include_prerelease: bool,
) -> Result {
    let is_worlds = match project.detect_project_type().await? {
        // we only can migrate legacy VRCSDK3 projects
        ProjectType::LegacyWorlds => true,
        ProjectType::LegacyAvatars => false,

        either => return Err(MigrateVpmError::ProjectTypeMismatch(either)),
    };

    info!(
        "Migrating {} Project",
        if is_worlds { "Worlds" } else { "Avatars" }
    );

    let mut adding_packages = vec![];
    let version_selector =
        VersionSelector::latest_for(Some(project.unity_version()), include_prerelease);

    // basic part: install SDK
    if is_worlds {
        adding_packages.push(
            collection
                .find_package_by_name("com.vrchat.worlds", version_selector)
                .ok_or(MigrateVpmError::VpmPackageNotFound("com.vrchat.worlds"))?,
        );
    } else {
        adding_packages.push(
            collection
                .find_package_by_name("com.vrchat.avatars", version_selector)
                .ok_or(MigrateVpmError::VpmPackageNotFound("com.vrchat.avatars"))?,
        );
    }

    // additional part: migrate VRChat-curated packages
    // we find legacy curated package by trying to install it and check if the project has legacy assets
    {
        let mut curated_packages = collection
            .get_curated_packages(version_selector)
            .collect::<Vec<_>>();

        debug!(
            "Trying to add the following curated packages to find legacy curated packages with legacyAssets: {:?}",
            curated_packages
        );

        let packages = project
            .add_package_request(
                collection,
                &curated_packages,
                AddPackageOperation::InstallToDependencies,
                include_prerelease,
            )
            .await?;

        let found_curated_packages = (packages.remove_legacy_folders().iter())
            .chain(packages.remove_legacy_files())
            .map(|(_, pkg)| *pkg)
            .collect::<HashSet<_>>();

        curated_packages.retain(|pkg| found_curated_packages.contains(pkg.name()));

        if curated_packages.is_empty() {
            info!("No legacy curated packages found");
        } else {
            for x in &curated_packages {
                info!("We found migrate curated package: {}", x.name());
            }
        }

        adding_packages.extend(curated_packages.into_iter());
    }

    // install packages. this also removes legacy VRCSDK and curated packages

    let request = project
        .add_package_request(
            collection,
            &adding_packages,
            AddPackageOperation::InstallToDependencies,
            include_prerelease,
        )
        .await?;

    project.apply_pending_changes(installer, request).await?;

    // update project settings
    let project_settings_path = "ProjectSettings/ProjectSettings.asset".as_ref();

    match project.io.open(project_settings_path).await {
        Ok(mut file) => {
            let mut buffer = String::new();
            file.read_to_string(&mut buffer).await?;
            drop(file);

            fn replace_setting(buffer: &mut String, setting: &str, old: &str, value: &str) -> bool {
                if let Some(pos) = buffer.find(setting) {
                    let before_ws = buffer[..pos]
                        .chars()
                        .last()
                        .map(|x| x.is_ascii_whitespace())
                        .unwrap_or(true);
                    let after_match = buffer[pos + setting.len()..].starts_with(old);
                    if before_ws && after_match {
                        let start = pos + setting.len();
                        let end = start + old.len();
                        buffer.replace_range(start..end, value);
                        return true;
                    }
                }
                false
            }

            let mut changed = false;

            changed |= replace_setting(
                &mut buffer,
                "enableNativePlatformBackendsForNewInputSystem: ",
                "0",
                "1",
            );

            changed |= replace_setting(&mut buffer, "disableOldInputManagerSupport: ", "1", "0");

            if changed {
                project
                    .io
                    .write_sync(project_settings_path, buffer.as_bytes())
                    .await?;
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            log::error!("ProjectSettings.asset not found");
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}

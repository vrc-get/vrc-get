use std::fmt;

use crate::io::ProjectIo;
use crate::unity_project::{pending_project_changes, PendingProjectChanges};
use crate::{PackageCollection, UnityProject, VersionSelector};

#[derive(Debug)]
#[non_exhaustive]
pub enum ReinstalPackagesError {
    NotInstalled { package_name: Box<str> },
    DependencyNotFound { dependency_name: Box<str> },
}

impl fmt::Display for ReinstalPackagesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReinstalPackagesError::NotInstalled { package_name } => {
                write!(f, "Package {} is not installed", package_name)
            }
            ReinstalPackagesError::DependencyNotFound { dependency_name } => write!(
                f,
                "Package {dependency_name} (maybe dependencies of the package) not found"
            ),
        }
    }
}

impl std::error::Error for ReinstalPackagesError {}

impl<IO: ProjectIo> UnityProject<IO> {
    pub async fn reinstall_request<'env>(
        &self,
        env: &'env impl PackageCollection,
        packages: &[&str],
    ) -> Result<PendingProjectChanges<'env>, ReinstalPackagesError> {
        let mut changes = pending_project_changes::Builder::new();

        for &package in packages {
            let Some(locked) = self.manifest.get_locked(package) else {
                return Err(ReinstalPackagesError::NotInstalled {
                    package_name: package.into(),
                });
            };

            let Some(pkg) = env.find_package_by_name(
                locked.name(),
                VersionSelector::specific_version(locked.version()),
            ) else {
                return Err(ReinstalPackagesError::DependencyNotFound {
                    dependency_name: locked.name().into(),
                });
            };

            changes.install_already_locked(pkg);
        }

        Ok(changes.build_resolve(self).await)
    }
}

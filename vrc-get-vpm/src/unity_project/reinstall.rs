use std::fmt;

use crate::unity_project::package_resolution::MissingDependencies;
use crate::unity_project::{PendingProjectChanges, pending_project_changes};
use crate::{PackageCollection, UnityProject, VersionSelector};

#[derive(Debug)]
#[non_exhaustive]
pub enum ReinstalPackagesError {
    NotInstalled { package_name: Box<str> },
    DependenciesNotFound { dependencies: Vec<Box<str>> },
}

impl fmt::Display for ReinstalPackagesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReinstalPackagesError::NotInstalled { package_name } => {
                write!(f, "Package {} is not installed", package_name)
            }
            ReinstalPackagesError::DependenciesNotFound { dependencies } => {
                write!(f, "Following dependencies are not found: ")?;
                let mut first = true;
                for dep in dependencies {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", dep)?;
                    first = false;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ReinstalPackagesError {}

impl UnityProject {
    pub async fn reinstall_request<'env>(
        &self,
        env: &'env impl PackageCollection,
        packages: &[&str],
    ) -> Result<PendingProjectChanges<'env>, ReinstalPackagesError> {
        let mut changes = pending_project_changes::Builder::new();
        let mut missing_dependencies = MissingDependencies::new();

        for &package in packages {
            let Some(locked) = self.manifest.get_locked(package) else {
                return Err(ReinstalPackagesError::NotInstalled {
                    package_name: package.into(),
                });
            };

            if let Some(pkg) = env.find_package_by_name(
                locked.name(),
                VersionSelector::specific_version(locked.version()),
            ) {
                changes.install_already_locked(pkg);
            } else {
                missing_dependencies.add(locked.name());
            };
        }

        if !missing_dependencies.is_empty() {
            return Err(ReinstalPackagesError::DependenciesNotFound {
                dependencies: missing_dependencies.into_vec(),
            });
        }

        Ok(changes.build_resolve(self).await)
    }
}

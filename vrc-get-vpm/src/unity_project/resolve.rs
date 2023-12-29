use std::{fmt, io};
use std::collections::HashSet;
use itertools::Itertools;
use crate::unity_project::{AddPackageErr, package_resolution, pending_project_changes, PendingProjectChanges};
use crate::{Environment, HttpClient, PackageCollection, UnityProject, VersionSelector};

#[derive(Debug)]
pub enum ResolvePackageErr {
    Io(io::Error),
    ConflictWithDependencies {
        /// conflicting package name
        conflict: String,
        /// the name of locked package
        dependency_name: String,
    },
    DependencyNotFound {
        dependency_name: String,
    },
}

impl fmt::Display for ResolvePackageErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolvePackageErr::Io(ioerr) => fmt::Display::fmt(ioerr, f),
            ResolvePackageErr::ConflictWithDependencies {
                conflict,
                dependency_name,
            } => write!(f, "{conflict} conflicts with {dependency_name}"),
            ResolvePackageErr::DependencyNotFound { dependency_name } => write!(
                f,
                "Package {dependency_name} (maybe dependencies of the package) not found"
            ),
        }
    }
}

impl std::error::Error for ResolvePackageErr {}

impl From<io::Error> for ResolvePackageErr {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<AddPackageErr> for ResolvePackageErr {
    fn from(value: AddPackageErr) -> Self {
        match value {
            AddPackageErr::DependencyNotFound { dependency_name } => {
                Self::DependencyNotFound { dependency_name }
            }
        }
    }
}

impl UnityProject {
    pub async fn resolve_request<'env>(
        &mut self,
        env: &'env Environment<impl HttpClient>,
    ) -> Result<PendingProjectChanges<'env>, AddPackageErr> {
        let mut changes = pending_project_changes::Builder::new();

        // first, process locked dependencies
        for dep in self.manifest.all_locked() {
            let pkg = env
                .find_package_by_name(dep.name(), VersionSelector::specific_version(dep.version()))
                .ok_or_else(|| AddPackageErr::DependencyNotFound {
                    dependency_name: dep.name().to_owned(),
                })?;

            changes.install_already_locked(pkg);
        }

        // then, process dependencies of unlocked packages.
        self.resolve_unlocked(env, &mut changes)?;

        Ok(changes.build_resolve(self).await)
    }

    fn resolve_unlocked<'env>(
        &self,
        env: &'env Environment<impl HttpClient>,
        changes: &mut pending_project_changes::Builder<'env>,
    ) -> Result<(), AddPackageErr> {
        if self.unlocked_packages().is_empty() {
            // if there are no unlocked packages, early return
            return Ok(());
        }

        // set of packages already installed as unlocked
        let unlocked_names: HashSet<_> = self
            .unlocked_packages()
            .iter()
            .filter_map(|(_, pkg)| pkg.as_ref())
            .map(|x| x.name())
            .collect();

        // then, process dependencies of unlocked packages.
        let dependencies_of_unlocked_packages = self
            .unlocked_packages
            .iter()
            .filter_map(|(_, pkg)| pkg.as_ref())
            .flat_map(|pkg| pkg.vpm_dependencies());

        let unlocked_dependencies_versions = dependencies_of_unlocked_packages
            .filter(|(k, _)| self.manifest.get_locked(k.as_str()).is_none()) // skip if already installed to locked
            .filter(|(k, _)| !unlocked_names.contains(k.as_str())) // skip if already installed as unlocked
            .into_group_map();

        if unlocked_dependencies_versions.is_empty() {
            // if no dependencies are to be installed, early return
            return Ok(());
        }

        let unlocked_dependencies = unlocked_dependencies_versions
            .into_iter()
            .map(|(pkg_name, ranges)| {
                env.find_package_by_name(
                    pkg_name,
                    VersionSelector::ranges_for(self.unity_version, &ranges),
                )
                    .ok_or_else(|| AddPackageErr::DependencyNotFound {
                        dependency_name: pkg_name.clone(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let allow_prerelease = unlocked_dependencies
            .iter()
            .any(|x| !x.version().pre.is_empty());

        let result = package_resolution::collect_adding_packages(
            self.manifest.dependencies(),
            self.manifest.all_locked(),
            |pkg| self.manifest.get_locked(pkg),
            self.unity_version(),
            env,
            unlocked_dependencies,
            allow_prerelease,
        )?;

        for x in result.new_packages {
            changes.install_to_locked(x);
        }

        for (package, conflicts_with) in result.conflicts {
            changes.conflict_multiple(package, conflicts_with);
        }

        Ok(())
    }
}

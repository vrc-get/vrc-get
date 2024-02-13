use crate::io::ProjectIo;
use crate::unity_project::pending_project_changes::RemoveReason;
use crate::unity_project::{package_resolution, PendingProjectChanges};
use crate::version::DependencyRange;
use crate::{PackageCollection, PackageInfo, UnityProject};
use std::fmt;

#[derive(Debug)]
#[non_exhaustive]
pub enum AddPackageErr {
    DependencyNotFound { dependency_name: Box<str> },
}

impl fmt::Display for AddPackageErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddPackageErr::DependencyNotFound { dependency_name } => write!(
                f,
                "Package {dependency_name} (maybe dependencies of the package) not found"
            ),
        }
    }
}

impl std::error::Error for AddPackageErr {}

// adding package
impl<IO: ProjectIo> UnityProject<IO> {
    /// Creates a new `AddPackageRequest` to add the specified packages.
    ///
    /// You should call `do_add_package_request` to apply the changes after confirming to the user.
    pub async fn add_package_request<'env>(
        &self,
        env: &'env impl PackageCollection,
        packages: Vec<PackageInfo<'env>>,
        to_dependencies: bool,
        allow_prerelease: bool,
    ) -> Result<PendingProjectChanges<'env>, AddPackageErr> {
        // if same or newer requested package is in locked dependencies,
        // just add requested version into dependencies
        let mut adding_packages = Vec::with_capacity(packages.len());

        let mut changes = super::pending_project_changes::Builder::new();

        for request in packages {
            if to_dependencies {
                let add_to_dependencies = self
                    .manifest
                    .get_dependency(request.name())
                    .and_then(|range| range.as_single_version())
                    .map(|full| &full < request.version())
                    .unwrap_or(true);

                if add_to_dependencies {
                    changes.add_to_dependencies(
                        request.name().into(),
                        DependencyRange::version(request.version().clone()),
                    );
                }
            }

            if self
                .manifest
                .get_locked(request.name())
                .map(|version| version.version() < request.version())
                .unwrap_or(true)
            {
                adding_packages.push(request);
            }
        }

        if adding_packages.is_empty() {
            // early return: nothing new to install
            return Ok(changes.build_no_resolve());
        }

        let result = package_resolution::collect_adding_packages(
            self.manifest.dependencies(),
            self.manifest.all_locked(),
            |pkg| self.manifest.get_locked(pkg),
            self.unity_version(),
            env,
            adding_packages,
            allow_prerelease,
        )?;

        for x in result.new_packages {
            changes.install_to_locked(x);
        }

        for (package, conflicts_with) in result.conflicts {
            changes.conflict_multiple(package, conflicts_with);
        }

        for name in result
            .found_legacy_packages
            .into_iter()
            .filter(|name| self.is_locked(name))
        {
            changes.remove(name, RemoveReason::Legacy);
        }

        Ok(changes.build_resolve(self).await)
    }
}

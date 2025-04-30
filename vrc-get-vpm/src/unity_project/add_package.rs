use crate::unity_project::package_resolution::MissingDependencies;
use crate::unity_project::pending_project_changes::RemoveReason;
use crate::unity_project::vpm_manifest::VpmManifest;
use crate::unity_project::{PendingProjectChanges, package_resolution};
use crate::version::DependencyRange;
use crate::{PackageCollection, PackageInfo, UnityProject};
use log::debug;
use std::fmt;

#[derive(Debug)]
#[non_exhaustive]
pub enum AddPackageErr {
    DependenciesNotFound { dependencies: Vec<Box<str>> },
    UpgradingNonLockedPackage { package_name: Box<str> },
    DowngradingNonLockedPackage { package_name: Box<str> },
    UpgradingWithDowngrade { package_name: Box<str> },
}

impl fmt::Display for AddPackageErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddPackageErr::DependenciesNotFound { dependencies } => {
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
            AddPackageErr::UpgradingNonLockedPackage { package_name } => write!(
                f,
                "Package {package_name} is not locked, so it cannot be upgraded"
            ),
            AddPackageErr::DowngradingNonLockedPackage { package_name } => write!(
                f,
                "Package {package_name} is not locked, so it cannot be downgraded"
            ),
            AddPackageErr::UpgradingWithDowngrade { package_name } => write!(
                f,
                "Package {package_name} is locked, so it cannot be downgraded"
            ),
        }
    }
}

impl std::error::Error for AddPackageErr {}

#[non_exhaustive]
#[derive(Debug)]
pub enum AddPackageOperation {
    InstallToDependencies,
    UpgradeLocked,
    Downgrade,
    AutoDetected,
}

// adding package
impl UnityProject {
    /// Creates a new `AddPackageRequest` to add the specified packages.
    ///
    /// You should call `apply_pending_changes` to apply the changes after confirming to the user.
    pub async fn add_package_request<'env>(
        &self,
        env: &'env impl PackageCollection,
        packages: &[PackageInfo<'env>],
        operation: AddPackageOperation,
        allow_prerelease: bool,
    ) -> Result<PendingProjectChanges<'env>, AddPackageErr> {
        // if same or newer requested package is in locked dependencies,
        // just add requested version into dependencies
        let mut adding_packages = Vec::with_capacity(packages.len());

        let mut changes = super::pending_project_changes::Builder::new();

        for &request in packages {
            debug!("Validating Package: {}", request.name());

            {
                fn install_to_dependencies<'env>(
                    request: PackageInfo<'env>,
                    this: &UnityProject,
                    adding_packages: &mut Vec<PackageInfo<'env>>,
                    changes: &mut super::pending_project_changes::Builder,
                ) -> Result<(), AddPackageErr> {
                    let add_to_dependencies = this
                        .manifest
                        .get_dependency(request.name())
                        .and_then(|range| range.as_single_version())
                        .map(|full| &full < request.version())
                        .unwrap_or(true);

                    if add_to_dependencies {
                        debug!("Adding package {} to dependencies", request.name());
                        changes.add_to_dependencies(
                            request.name().into(),
                            DependencyRange::version(request.version().clone()),
                        );
                    }

                    check_and_add_adding_package(request, adding_packages, &this.manifest);

                    Ok(())
                }

                fn upgrade_locked<'env>(
                    request: PackageInfo<'env>,
                    this: &UnityProject,
                    adding_packages: &mut Vec<PackageInfo<'env>>,
                    _changes: &mut super::pending_project_changes::Builder,
                ) -> Result<(), AddPackageErr> {
                    check_and_add_adding_package(request, adding_packages, &this.manifest);
                    Ok(())
                }

                fn downgrade<'env>(
                    request: PackageInfo<'env>,
                    this: &UnityProject,
                    adding_packages: &mut Vec<PackageInfo<'env>>,
                    changes: &mut super::pending_project_changes::Builder,
                ) -> Result<(), AddPackageErr> {
                    let downgrade_dependencies = this
                        .manifest
                        .get_dependency(request.name())
                        .map(|range| !range.matches(request.version()))
                        .unwrap_or(false);

                    if downgrade_dependencies {
                        debug!(
                            "Downgrading package {} to version {} in dependencies",
                            request.name(),
                            request.version()
                        );
                        // downgrade to >= requested version
                        changes.add_to_dependencies(
                            request.name().into(),
                            DependencyRange::version(request.version().clone()),
                        );
                    }

                    // always add to adding_packages since downloading
                    debug!(
                        "Adding package {} to locked packages at version {}",
                        request.name(),
                        request.version()
                    );
                    adding_packages.push(request);

                    Ok(())
                }

                match operation {
                    AddPackageOperation::InstallToDependencies => {
                        install_to_dependencies(request, self, &mut adding_packages, &mut changes)?;
                    }
                    AddPackageOperation::UpgradeLocked => {
                        if self.manifest.get_locked(request.name()).is_none() {
                            // if package is not locked, it cannot be updated
                            return Err(AddPackageErr::UpgradingNonLockedPackage {
                                package_name: request.name().into(),
                            });
                        }

                        upgrade_locked(request, self, &mut adding_packages, &mut changes)?;
                    }
                    AddPackageOperation::Downgrade => {
                        let Some(locked_version) = self.manifest.get_locked(request.name()) else {
                            // if package is not locked, it cannot be updated
                            return Err(AddPackageErr::DowngradingNonLockedPackage {
                                package_name: request.name().into(),
                            });
                        };

                        if locked_version.version() < request.version() {
                            // if the locked version is older than the requested version,
                            // it cannot be downgraded
                            return Err(AddPackageErr::UpgradingWithDowngrade {
                                package_name: request.name().into(),
                            });
                        }

                        downgrade(request, self, &mut adding_packages, &mut changes)?;
                    }
                    AddPackageOperation::AutoDetected => {
                        match self.manifest.get_locked(request.name()) {
                            None => {
                                // not installed: install to dependencies

                                debug!("Adding package {} to dependencies", request.name());

                                install_to_dependencies(
                                    request,
                                    self,
                                    &mut adding_packages,
                                    &mut changes,
                                )?;
                            }
                            // already installed: upgrade, downgrade, or reinstall
                            Some(locked) => match locked.version().cmp(request.version()) {
                                std::cmp::Ordering::Less => {
                                    // upgrade
                                    debug!(
                                        "Upgrading package {} to version {}",
                                        request.name(),
                                        request.version()
                                    );
                                    upgrade_locked(
                                        request,
                                        self,
                                        &mut adding_packages,
                                        &mut changes,
                                    )?;
                                }
                                std::cmp::Ordering::Equal => {
                                    // reinstall
                                    debug!(
                                        "Reinstalling package {} at version {}",
                                        request.name(),
                                        request.version()
                                    );
                                    adding_packages.push(request);
                                }
                                std::cmp::Ordering::Greater => {
                                    // downgrade
                                    debug!(
                                        "Downgrading package {} to version {}",
                                        request.name(),
                                        request.version()
                                    );
                                    downgrade(request, self, &mut adding_packages, &mut changes)?;
                                }
                            },
                        }
                    }
                }
            }

            fn check_and_add_adding_package<'env>(
                request: PackageInfo<'env>,
                adding_packages: &mut Vec<PackageInfo<'env>>,
                manifest: &VpmManifest,
            ) {
                if manifest
                    .get_locked(request.name())
                    .map(|version| version.version() < request.version())
                    .unwrap_or(true)
                {
                    debug!(
                        "Adding package {} to locked packages at version {}",
                        request.name(),
                        request.version()
                    );
                    adding_packages.push(request);
                } else {
                    debug!(
                        "Package {} is already locked at newer version than {}: version {}",
                        request.name(),
                        request.version(),
                        manifest.get_locked(request.name()).unwrap().version()
                    );
                }
            }
        }

        debug!("Validation finished");

        if adding_packages.is_empty() {
            debug!("No new packages to add, returning early");
            // early return: nothing new to install
            return Ok(changes.build_no_resolve());
        }

        debug!("Resolving dependencies");

        let mut missing_dependencies = MissingDependencies::new();
        let result = package_resolution::collect_adding_packages(
            self.manifest.dependencies().map(|(name, original_range)| {
                if let Some(new_range) = changes.get_dependencies(name) {
                    (name, new_range)
                } else {
                    (name, original_range)
                }
            }),
            self.manifest.all_locked(),
            self.unlocked_packages.iter(),
            |pkg| self.manifest.get_locked(pkg),
            Some(self.unity_version()),
            env,
            adding_packages,
            allow_prerelease,
            &mut missing_dependencies,
        );
        if !missing_dependencies.is_empty() {
            return Err(AddPackageErr::DependenciesNotFound {
                dependencies: missing_dependencies.into_vec(),
            });
        }

        debug!("Resolving finished");

        for pkg in result.new_packages {
            debug!("Installing package {}@{}", pkg.name(), pkg.version());
            changes.install_to_locked(pkg);

            for (dir, _) in self.unlocked_packages.iter().filter(|(dir, unlocked)| {
                dir.as_ref() == pkg.name()
                    || unlocked
                        .as_ref()
                        .map(|x| x.name() == pkg.name())
                        .unwrap_or(false)
            }) {
                changes.unlocked_installation_conflict(pkg.name().into(), dir.clone());
            }
        }

        for (package, conflicts_with) in result.conflicts {
            debug!("package {} conflicts with {:?}", package, conflicts_with);
            changes.conflict_multiple(package, conflicts_with);
        }

        for name in result
            .found_legacy_packages
            .into_iter()
            .filter(|name| self.is_locked(name))
        {
            debug!("removing legacy package {}", name);
            changes.remove(name, RemoveReason::Legacy);
        }

        debug!("Building changes (finding legacy assets, checking conflicts)");

        let changes = changes.build_resolve(self).await;

        debug!("Resolving finished");

        Ok(changes)
    }
}

mod add_package;
mod find_legacy_assets;
mod package_resolution;
pub mod pending_project_changes;
mod remove_package;
mod vpm_manifest;

use crate::structs::package::PackageJson;
use crate::unity_project::vpm_manifest::VpmManifest;
use crate::utils::{load_json_or_default, try_load_json, PathBufExt};
use crate::version::{UnityVersion, Version, VersionRange};
use crate::{Environment, PackageInfo, VersionSelector};
use futures::future::try_join_all;
use futures::prelude::*;
use indexmap::IndexMap;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::{env, fmt, io};
use tokio::fs::{read_dir, remove_dir_all, DirEntry, File};
use tokio::io::AsyncReadExt;

// note: this module only declares basic small operations.
// there are module for each complex operations.

use crate::traits::{HttpClient, PackageCollection};
use crate::unity_project::pending_project_changes::PackageChange;
pub use add_package::AddPackageErr;
pub use pending_project_changes::PendingProjectChanges;

#[derive(Debug)]
pub struct UnityProject {
    /// path to project folder.
    project_dir: PathBuf,
    /// manifest.json
    manifest: VpmManifest,
    /// unity version parsed
    unity_version: Option<UnityVersion>,
    /// packages installed in the directory but not locked in vpm-manifest.json
    unlocked_packages: Vec<(String, Option<PackageJson>)>,
    installed_packages: HashMap<String, PackageJson>,
}

// basic lifecycle
impl UnityProject {
    pub async fn find_unity_project(unity_project: Option<PathBuf>) -> io::Result<UnityProject> {
        let unity_found = unity_project
            .ok_or(())
            .or_else(|_| UnityProject::find_unity_project_path())?;

        log::debug!(
            "initializing UnityProject with unity folder {}",
            unity_found.display()
        );

        let manifest = unity_found.join("Packages").joined("vpm-manifest.json");
        let manifest = VpmManifest::from(&manifest).await?;

        let mut installed_packages = HashMap::new();
        let mut unlocked_packages = vec![];

        let mut dir_reading = read_dir(unity_found.join("Packages")).await?;
        while let Some(dir_entry) = dir_reading.next_entry().await? {
            let read = Self::try_read_unlocked_package(dir_entry).await;
            let mut is_installed = false;
            if let Some(parsed) = &read.1 {
                if parsed.name() == read.0 && manifest.get_locked(parsed.name()).is_some() {
                    is_installed = true;
                }
            }
            if is_installed {
                installed_packages.insert(read.0, read.1.unwrap());
            } else {
                unlocked_packages.push(read);
            }
        }

        let unity_version = Self::try_read_unity_version(&unity_found).await;

        Ok(UnityProject {
            project_dir: unity_found,
            manifest,
            unity_version,
            unlocked_packages,
            installed_packages,
        })
    }

    async fn try_read_unlocked_package(dir_entry: DirEntry) -> (String, Option<PackageJson>) {
        let package_path = dir_entry.path();
        let name = package_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let package_json_path = package_path.join("package.json");
        let parsed = try_load_json::<PackageJson>(&package_json_path)
            .await
            .ok()
            .flatten();
        (name, parsed)
    }

    fn find_unity_project_path() -> io::Result<PathBuf> {
        let mut candidate = env::current_dir()?;

        loop {
            candidate.push("Packages");
            candidate.push("vpm-manifest.json");

            if candidate.exists() {
                log::debug!("vpm-manifest.json found at {}", candidate.display());
                // if there's vpm-manifest.json, it's project path
                candidate.pop();
                candidate.pop();
                return Ok(candidate);
            }

            // replace vpm-manifest.json -> manifest.json
            candidate.pop();
            candidate.push("manifest.json");

            if candidate.exists() {
                log::debug!("manifest.json found at {}", candidate.display());
                // if there's manifest.json (which is manifest.json), it's project path
                candidate.pop();
                candidate.pop();
                return Ok(candidate);
            }

            // remove Packages/manifest.json
            candidate.pop();
            candidate.pop();

            log::debug!("Unity Project not found on {}", candidate.display());

            // go to parent dir
            if !candidate.pop() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Unity project Not Found",
                ));
            }
        }
    }

    async fn try_read_unity_version(unity_project: &Path) -> Option<UnityVersion> {
        let project_version_file = unity_project
            .join("ProjectSettings")
            .joined("ProjectVersion.txt");

        let mut project_version_file = match File::open(project_version_file).await {
            Ok(file) => file,
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                log::error!("ProjectVersion.txt not found");
                return None;
            }
            Err(e) => {
                log::error!("opening ProjectVersion.txt failed with error: {e}");
                return None;
            }
        };

        let mut buffer = String::new();

        if let Err(e) = project_version_file.read_to_string(&mut buffer).await {
            log::error!("reading ProjectVersion.txt failed with error: {e}");
            return None;
        };

        let Some((_, version_info)) = buffer.split_once("m_EditorVersion:") else {
            log::error!("m_EditorVersion not found in ProjectVersion.txt");
            return None;
        };

        let version_info_end = version_info
            .find(|x: char| x == '\r' || x == '\n')
            .unwrap_or(version_info.len());
        let version_info = &version_info[..version_info_end];
        let version_info = version_info.trim();

        let Some(unity_version) = UnityVersion::parse(version_info) else {
            log::error!("failed to unity version in ProjectVersion.txt ({version_info})");
            return None;
        };

        Some(unity_version)
    }

    pub async fn save(&mut self) -> io::Result<()> {
        self.manifest
            .save_to(
                &self
                    .project_dir
                    .join("Packages")
                    .joined("vpm-manifest.json"),
            )
            .await
    }
}

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

pub struct ResolveResult<'env> {
    installed_from_locked: Vec<PackageInfo<'env>>,
    installed_from_unlocked_dependencies: Vec<PackageInfo<'env>>,
}

impl<'env> ResolveResult<'env> {
    pub fn installed_from_locked(&self) -> &[PackageInfo<'env>] {
        &self.installed_from_locked
    }

    pub fn installed_from_unlocked_dependencies(&self) -> &[PackageInfo<'env>] {
        &self.installed_from_unlocked_dependencies
    }
}

impl UnityProject {
    pub async fn resolve<'env>(
        &mut self,
        env: &'env Environment<impl HttpClient>,
    ) -> Result<ResolveResult<'env>, ResolvePackageErr> {
        let changes = match self.resolve_request(env).await {
            Ok(changes) => changes,
            Err(AddPackageErr::DependencyNotFound { dependency_name }) => {
                return Err(ResolvePackageErr::DependencyNotFound { dependency_name })
            }
        };

        if let Some((conflict, dep)) = changes
            .conflicts
            .iter()
            .filter_map(|(name, x)| x.conflicting_packages().first().map(|x| (name, x)))
            .next()
        {
            return Err(ResolvePackageErr::ConflictWithDependencies {
                conflict: conflict.to_owned(),
                dependency_name: dep.to_owned(),
            });
        }

        let mut installed_from_locked = Vec::new();
        let mut installed_from_unlocked_dependencies = Vec::new();

        for change in changes.package_changes.values() {
            match change {
                PackageChange::Install(install) if install.is_adding_to_locked() => {
                    // adding to locked: for unlocked
                    installed_from_unlocked_dependencies.push(install.install_package().unwrap());
                }
                PackageChange::Install(install) if install.install_package().is_some() => {
                    // already in locked: for locked
                    installed_from_locked.push(install.install_package().unwrap());
                }
                PackageChange::Install(_) => (),
                PackageChange::Remove(_) => (),
            }
        }

        Ok(ResolveResult {
            installed_from_locked,
            installed_from_unlocked_dependencies,
        })
    }

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

    /// Remove specified package from self project.
    ///
    /// This doesn't look packages not listed in vpm-maniefst.json.
    pub async fn mark_and_sweep(&mut self) -> io::Result<HashSet<String>> {
        let removed_packages = self
            .manifest
            .mark_and_sweep_packages(&self.unlocked_packages);

        try_join_all(removed_packages.iter().map(|name| {
            remove_dir_all(self.project_dir.join("Packages").joined(name)).map(|x| match x {
                Ok(()) => Ok(()),
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e),
            })
        }))
        .await?;

        Ok(removed_packages)
    }
}

// accessors
impl UnityProject {
    pub fn locked_packages(&self) -> impl Iterator<Item = LockedDependencyInfo> {
        self.manifest.all_locked()
    }

    pub fn dependencies(&self) -> impl Iterator<Item = &str> {
        self.manifest.dependencies().map(|(name, _)| name)
    }

    pub(crate) fn get_locked(&self, name: &str) -> Option<LockedDependencyInfo> {
        self.manifest.get_locked(name)
    }

    pub fn is_locked(&self, name: &str) -> bool {
        self.manifest.get_locked(name).is_some()
    }

    pub fn all_packages(&self) -> impl Iterator<Item = LockedDependencyInfo> {
        let dependencies_locked = self.manifest.all_locked();

        let dependencies_unlocked = self
            .unlocked_packages
            .iter()
            .filter_map(|(_, json)| json.as_ref())
            .map(|x| LockedDependencyInfo::new(x.name(), x.version(), x.vpm_dependencies()));

        dependencies_locked.chain(dependencies_unlocked)
    }

    pub fn unlocked_packages(&self) -> &[(String, Option<PackageJson>)] {
        &self.unlocked_packages
    }

    pub fn get_installed_package(&self, name: &str) -> Option<&PackageJson> {
        self.installed_packages.get(name)
    }

    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    pub fn unity_version(&self) -> Option<UnityVersion> {
        self.unity_version
    }
}

pub struct LockedDependencyInfo<'a> {
    name: &'a str,
    version: &'a Version,
    dependencies: &'a IndexMap<String, VersionRange>,
}

impl<'a> LockedDependencyInfo<'a> {
    fn new(
        name: &'a str,
        version: &'a Version,
        dependencies: &'a IndexMap<String, VersionRange>,
    ) -> Self {
        Self {
            name,
            version,
            dependencies,
        }
    }

    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn version(&self) -> &'a Version {
        self.version
    }

    pub fn dependencies(&self) -> &'a IndexMap<String, VersionRange> {
        self.dependencies
    }
}

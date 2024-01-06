mod add_package;
mod find_legacy_assets;
mod migrate_unity_2022;
mod package_resolution;
pub mod pending_project_changes;
mod remove_package;
mod resolve;
mod upm_manifest;
mod vpm_manifest;

use crate::structs::package::PackageJson;
use crate::unity_project::upm_manifest::UpmManifest;
use crate::unity_project::vpm_manifest::VpmManifest;
use crate::utils::{load_json_or_default, try_load_json, PathBufExt};
use crate::version::{UnityVersion, Version, VersionRange};
use indexmap::IndexMap;
use log::debug;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, io};
use tokio::fs::{read_dir, DirEntry, File};
use tokio::io::AsyncReadExt;

// note: this module only declares basic small operations.
// there are module for each complex operations.

pub use add_package::AddPackageErr;
pub use migrate_unity_2022::MigrateUnity2022Error;
pub use pending_project_changes::PendingProjectChanges;
pub use resolve::ResolvePackageErr;

#[derive(Debug)]
pub struct UnityProject {
    /// path to project folder.
    project_dir: PathBuf,
    /// vpm-manifest.json
    manifest: VpmManifest,
    // manifest.json
    upm_manifest: UpmManifest,
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
        let upm_manifest = unity_found.join("Packages").joined("manifest.json");
        let upm_manifest = UpmManifest::from(&upm_manifest).await?;

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

        let project = UnityProject {
            project_dir: unity_found,
            manifest,
            upm_manifest,
            unity_version,
            unlocked_packages,
            installed_packages,
        };

        debug!("UnityProject initialized: {:#?}", project);

        Ok(project)
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
            .await?;
        self.upm_manifest
            .save(&self.project_dir.join("Packages").joined("manifest.json"))
            .await?;
        Ok(())
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

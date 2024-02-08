mod add_package;
mod call_unity;
mod find_legacy_assets;
mod migrate_unity_2022;
mod package_resolution;
pub mod pending_project_changes;
mod remove_package;
mod resolve;
mod upm_manifest;
mod vpm_manifest;

use crate::io;
use crate::unity_project::upm_manifest::UpmManifest;
use crate::unity_project::vpm_manifest::VpmManifest;
use crate::utils::{try_load_json, PathBufExt};
use crate::version::{UnityVersion, Version, VersionRange};
use futures::future::try_join;
use futures::prelude::*;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// note: this module only declares basic small operations.
// there are module for each complex operations.

use crate::io::{DirEntry, FileSystemProjectIo, ProjectIo};
use crate::PackageJson;
pub use add_package::AddPackageErr;
pub use call_unity::ExecuteUnityError;
pub use migrate_unity_2022::MigrateUnity2022Error;
pub use pending_project_changes::PendingProjectChanges;
pub use resolve::ResolvePackageErr;

#[derive(Debug)]
pub struct UnityProject<IO: ProjectIo> {
    io: IO,
    /// vpm-manifest.json
    manifest: VpmManifest,
    // manifest.json
    upm_manifest: UpmManifest,
    /// unity version parsed
    unity_version: Option<UnityVersion>,
    /// packages installed in the directory but not locked in vpm-manifest.json
    unlocked_packages: Vec<(Box<str>, Option<PackageJson>)>,
    installed_packages: HashMap<Box<str>, PackageJson>,
}

// basic lifecycle
impl<IO: ProjectIo> UnityProject<IO> {
    pub async fn load(io: IO) -> io::Result<Self> {
        let manifest = VpmManifest::load(&io).await?;
        let upm_manifest = UpmManifest::load(&io).await?;

        let mut installed_packages = HashMap::new();
        let mut unlocked_packages = vec![];

        let mut dir_reading = io.read_dir("Packages".as_ref()).await?;
        while let Some(dir_entry) = dir_reading.try_next().await? {
            let read = Self::try_read_unlocked_package(&io, dir_entry).await;
            let mut is_installed = false;
            if let Some(parsed) = &read.1 {
                if parsed.name() == read.0.as_ref() && manifest.get_locked(parsed.name()).is_some()
                {
                    is_installed = true;
                }
            }
            if is_installed {
                installed_packages.insert(read.0, read.1.unwrap());
            } else {
                unlocked_packages.push(read);
            }
        }

        let unity_version = Self::try_read_unity_version(&io).await;

        Ok(Self {
            io,
            manifest,
            upm_manifest,
            unity_version,
            unlocked_packages,
            installed_packages,
        })
    }
}

impl<IO: ProjectIo> UnityProject<IO> {
    async fn try_read_unlocked_package(
        io: &IO,
        dir_entry: IO::DirEntry,
    ) -> (Box<str>, Option<PackageJson>) {
        let name = dir_entry.file_name().to_string_lossy().into();
        let package_json_path = PathBuf::from("Packages")
            .joined(dir_entry.file_name())
            .joined("package.json");
        let parsed = try_load_json::<PackageJson>(io, &package_json_path)
            .await
            .ok()
            .flatten();
        (name, parsed)
    }

    async fn try_read_unity_version(io: &IO) -> Option<UnityVersion> {
        let mut project_version_file =
            match io.open("ProjectSettings/ProjectVersion.txt".as_ref()).await {
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
        try_join(
            self.manifest.save(&self.io),
            self.upm_manifest.save(&self.io),
        )
        .await?;
        Ok(())
    }
}

// accessors
impl<IO: ProjectIo> UnityProject<IO> {
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

    pub fn unlocked_packages(&self) -> &[(Box<str>, Option<PackageJson>)] {
        &self.unlocked_packages
    }

    pub fn get_installed_package(&self, name: &str) -> Option<&PackageJson> {
        self.installed_packages.get(name)
    }

    pub fn all_installed_packages(&self) -> impl Iterator<Item = &PackageJson> {
        self.installed_packages.values().chain(
            self.unlocked_packages
                .iter()
                .filter_map(|(_, json)| json.as_ref()),
        )
    }

    pub fn unity_version(&self) -> Option<UnityVersion> {
        self.unity_version
    }
}

impl<IO: FileSystemProjectIo + ProjectIo> UnityProject<IO> {
    pub fn project_dir(&self) -> &Path {
        self.io.location()
    }
}

#[derive(Clone)]
pub struct LockedDependencyInfo<'a> {
    name: &'a str,
    version: &'a Version,
    dependencies: &'a IndexMap<Box<str>, VersionRange>,
}

impl<'a> LockedDependencyInfo<'a> {
    fn new(
        name: &'a str,
        version: &'a Version,
        dependencies: &'a IndexMap<Box<str>, VersionRange>,
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

    pub fn dependencies(&self) -> &'a IndexMap<Box<str>, VersionRange> {
        self.dependencies
    }
}

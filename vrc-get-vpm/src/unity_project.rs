mod add_package;
mod find_legacy_assets;
mod migrate_unity_2022;
mod migrate_vpm;
mod package_resolution;
pub mod pending_project_changes;
mod project_type;
mod remove_package;
mod resolve;
mod upm_manifest;
mod vpm_manifest;

use crate::unity_project::upm_manifest::UpmManifest;
use crate::unity_project::vpm_manifest::VpmManifest;
use crate::utils::{try_load_json, PathBufExt};
use crate::version::{UnityVersion, Version, VersionRange};
use crate::{io, PackageManifest};
use futures::future::try_join;
use futures::prelude::*;
use indexmap::IndexMap;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// note: this module only declares basic small operations.
// there are module for each complex operations.

use crate::io::{DirEntry, FileSystemProjectIo, ProjectIo};
use crate::package_manifest::LooseManifest;
pub use add_package::AddPackageErr;
pub use add_package::AddPackageOperation;
pub use migrate_unity_2022::MigrateUnity2022Error;
pub use migrate_vpm::MigrateVpmError;
pub use pending_project_changes::PendingProjectChanges;
pub use remove_package::RemovePackageErr;
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
    /// unity revision parsed
    unity_revision: Option<String>,
    /// packages installed in the directory but not locked in vpm-manifest.json
    unlocked_packages: Vec<(Box<str>, Option<PackageManifest>)>,
    /// packages installed in the directory and licked in vpm-manifest.json
    installed_packages: HashMap<Box<str>, PackageManifest>,
}

// basic lifecycle
impl<IO: ProjectIo> UnityProject<IO> {
    pub async fn load(io: IO) -> io::Result<Self> {
        let manifest = VpmManifest::load(&io).await?;
        let upm_manifest = UpmManifest::load(&io).await?;

        let mut installed_packages = HashMap::new();
        let mut unlocked_packages = vec![];

        match io.read_dir("Packages".as_ref()).await {
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                log::error!("Packages directory not found");
            }
            Err(e) => {
                return Err(e);
            }
            Ok(mut dir_reading) => {
                while let Some(dir_entry) = dir_reading.try_next().await? {
                    if !dir_entry.file_type().await?.is_dir() {
                        continue;
                    }
                    let read = Self::try_read_unlocked_package(&io, dir_entry).await;
                    let mut is_installed = false;
                    if let Some(parsed) = &read.1 {
                        if parsed.name() == read.0.as_ref()
                            && manifest.get_locked(parsed.name()).is_some()
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
            }
        }

        let (unity_version, unity_revision) = Self::try_read_unity_version(&io).await;

        Ok(Self {
            io,
            manifest,
            upm_manifest,
            unity_version,
            unity_revision,
            unlocked_packages,
            installed_packages,
        })
    }
}

impl<IO: ProjectIo> UnityProject<IO> {
    async fn try_read_unlocked_package(
        io: &IO,
        dir_entry: IO::DirEntry,
    ) -> (Box<str>, Option<PackageManifest>) {
        let name = dir_entry.file_name().to_string_lossy().into();
        let package_json_path = PathBuf::from("Packages")
            .joined(dir_entry.file_name())
            .joined("package.json");
        let parsed = try_load_json::<LooseManifest>(io, &package_json_path)
            .await
            .ok()
            .flatten();
        (name, parsed.map(|x| x.0))
    }

    async fn try_read_unity_version(io: &IO) -> (Option<UnityVersion>, Option<String>) {
        let mut project_version_file =
            match io.open("ProjectSettings/ProjectVersion.txt".as_ref()).await {
                Ok(file) => file,
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                    log::error!("ProjectVersion.txt not found");
                    return (None, None);
                }
                Err(e) => {
                    log::error!("opening ProjectVersion.txt failed with error: {e}");
                    return (None, None);
                }
            };

        let mut buffer = String::new();

        if let Err(e) = project_version_file.read_to_string(&mut buffer).await {
            log::error!("reading ProjectVersion.txt failed with error: {e}");
            return (None, None);
        };

        let unity_version = match Self::find_attribute(buffer.as_str(), "m_EditorVersion:") {
            None => None,
            Some(version_info) => {
                let parsed = UnityVersion::parse(version_info);
                if parsed.is_none() {
                    log::error!("failed to parse m_EditorVersion in ProjectVersion.txt");
                }
                parsed
            }
        };

        let revision = match Self::find_attribute(buffer.as_str(), "m_EditorVersionWithRevision:") {
            None => None,
            Some(version_info) => {
                let parsed = Self::parse_version_with_revision(version_info);
                if parsed.is_none() {
                    log::error!(
                        "failed to parse m_EditorVersionWithRevision in ProjectVersion.txt"
                    );
                }
                parsed
            }
        };

        (unity_version, revision.map(|x| x.to_string()))
    }

    fn find_attribute<'a>(buffer: &'a str, attribute: &str) -> Option<&'a str> {
        let (_, version_info) = buffer.split_once(attribute)?;
        let version_info_end = version_info
            .find(['\r', '\n'])
            .unwrap_or(version_info.len());
        let version_info = &version_info[..version_info_end];
        let version_info = version_info.trim();
        Some(version_info)
    }

    fn parse_version_with_revision(version_info: &str) -> Option<&str> {
        let (_version, revision) = version_info.split_once('(')?;
        let (revision, _) = revision.split_once(')')?;

        Some(revision)
    }

    pub async fn is_valid(&self) -> bool {
        self.unity_version.is_some()
    }

    pub fn io(&self) -> &IO {
        &self.io
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

    pub fn get_locked(&self, name: &str) -> Option<LockedDependencyInfo> {
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
            .map(|x| LockedDependencyInfo::new(x.name(), x.version(), Some(x.vpm_dependencies())));

        dependencies_locked.chain(dependencies_unlocked)
    }

    pub fn unlocked_packages(&self) -> &[(Box<str>, Option<PackageManifest>)] {
        &self.unlocked_packages
    }

    pub fn installed_packages(&self) -> impl Iterator<Item = (&str, &PackageManifest)> {
        self.installed_packages
            .iter()
            .map(|(key, value)| (key.as_ref(), value))
    }

    pub fn get_installed_package(&self, name: &str) -> Option<&PackageManifest> {
        self.installed_packages.get(name)
    }

    pub fn all_installed_packages(&self) -> impl Iterator<Item = &PackageManifest> {
        self.installed_packages.values().chain(
            self.unlocked_packages
                .iter()
                .filter_map(|(_, json)| json.as_ref()),
        )
    }

    pub fn unity_version(&self) -> Option<UnityVersion> {
        self.unity_version
    }

    pub fn unity_revision(&self) -> Option<&str> {
        self.unity_revision.as_deref()
    }

    pub fn has_upm_package(&self, name: &str) -> bool {
        self.upm_manifest.get_dependency(name).is_some()
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
        dependencies: Option<&'a IndexMap<Box<str>, VersionRange>>,
    ) -> Self {
        lazy_static! {
            static ref EMPTY_DEPENDENCIES: IndexMap<Box<str>, VersionRange> = IndexMap::new();
        }
        Self {
            name,
            version,
            dependencies: dependencies.unwrap_or(&*EMPTY_DEPENDENCIES),
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

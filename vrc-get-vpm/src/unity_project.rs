mod add_package;
mod find_legacy_assets;
mod migrate_unity_2022;
mod migrate_vpm;
mod package_resolution;
pub mod pending_project_changes;
mod project_type;
mod reinstall;
mod remove_package;
mod resolve;
mod upm_manifest;
mod vpm_manifest;

use crate::unity_project::upm_manifest::UpmManifest;
use crate::unity_project::vpm_manifest::VpmManifest;
use crate::utils::{PathBufExt, try_load_json};
use crate::version::{DependencyRange, UnityVersion, Version, VersionRange};
use crate::{PackageManifest, io};
use futures::future::try_join;
use futures::prelude::*;
use indexmap::IndexMap;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// note: this module only declares basic small operations.
// there are module for each complex operations.

use crate::io::{DefaultProjectIo, DirEntry, IoTrait, TokioDirEntry};
use crate::package_manifest::LooseManifest;
pub use add_package::AddPackageErr;
pub use add_package::AddPackageOperation;
pub use migrate_unity_2022::MigrateUnity2022Error;
pub use migrate_vpm::MigrateVpmError;
pub use pending_project_changes::PendingProjectChanges;
pub use reinstall::ReinstalPackagesError;
pub use remove_package::RemovePackageErr;
pub use resolve::ResolvePackageErr;

#[derive(Debug)]
pub struct UnityProject {
    io: DefaultProjectIo,
    /// vpm-manifest.json
    manifest: VpmManifest,
    // manifest.json
    upm_manifest: UpmManifest,
    /// unity version parsed
    unity_version: UnityVersion,
    /// unity revision parsed
    unity_revision: Option<String>,
    /// packages installed in the directory but not locked in vpm-manifest.json
    unlocked_packages: Vec<(Box<str>, Option<PackageManifest>)>,
    /// packages installed in the directory and licked in vpm-manifest.json
    installed_packages: HashMap<Box<str>, PackageManifest>,
}

// basic lifecycle
impl UnityProject {
    pub async fn load(io: DefaultProjectIo) -> io::Result<Self> {
        let manifest = VpmManifest::load(&io).await?;
        let upm_manifest = UpmManifest::load(&io).await?;

        fix_for_previous_vrc_get_bug(&io, &manifest).await.ok();

        // In previous version of vrc-get, we might emit "3.2-3.8" for "3.2 - 3.8" range notation.
        // This is parseable with ALCOM's implementation but not with SemVer.NET.
        // TODO: remove this fix in the future
        async fn fix_for_previous_vrc_get_bug(
            io: &DefaultProjectIo,
            parsed_manifest: &VpmManifest,
        ) -> io::Result<()> {
            const MANIFEST_PATH: &str = "Packages/vpm-manifest.json";
            const BAD_CONFIG: &[u8] = br##""3.2-3.8""##;

            let mut file = io.open(MANIFEST_PATH.as_ref()).await?;
            let mut buffer = vec![];
            file.read_to_end(&mut buffer).await?;
            if buffer.windows(BAD_CONFIG.len()).any(|s| s == BAD_CONFIG) {
                // We found bad notation. replace with fixed one.
                io.write_atomic(MANIFEST_PATH.as_ref(), &parsed_manifest.to_json()?)
                    .await?;
            }

            Ok(())
        }

        let mut installed_packages = HashMap::new();
        let mut unlocked_packages = vec![];

        match io.read_dir("Packages".as_ref()).await {
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                log::warn!("Packages directory not found");
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
                    if let Some(parsed) = &read.1
                        && parsed.name() == read.0.as_ref()
                        && manifest.get_locked(parsed.name()).is_some()
                    {
                        is_installed = true;
                    }
                    if is_installed {
                        installed_packages.insert(read.0, read.1.unwrap());
                    } else {
                        unlocked_packages.push(read);
                    }
                }
            }
        }

        let (unity_version, unity_revision) = Self::read_unity_version(&io).await?;

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

impl UnityProject {
    async fn try_read_unlocked_package(
        io: &DefaultProjectIo,
        dir_entry: TokioDirEntry,
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

    async fn read_unity_version(
        io: &DefaultProjectIo,
    ) -> io::Result<(UnityVersion, Option<String>)> {
        let mut buffer = String::new();

        io.open("ProjectSettings/ProjectVersion.txt".as_ref())
            .await
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Error opening ProjectVersion.txt: {e}"),
                )
            })?
            .read_to_string(&mut buffer)
            .await
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Error reading ProjectVersion.txt: {e}"),
                )
            })?;

        let Some(unity_version) = Self::find_attribute(buffer.as_str(), "m_EditorVersion:") else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Failed to parse m_EditorVersion: No m_EditorVersion was found",
            ));
        };
        let Some(unity_version) = UnityVersion::parse(unity_version) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse m_EditorVersion: {unity_version}"),
            ));
        };

        let revision = Self::find_attribute(buffer.as_str(), "m_EditorVersionWithRevision:")
            .map(|version_info| {
                Self::parse_version_with_revision(version_info).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Failed to parse m_EditorVersionWithRevision",
                    )
                })
            })
            .transpose()?
            .map(ToOwned::to_owned);

        Ok((unity_version, revision))
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

    pub fn io(&self) -> &DefaultProjectIo {
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
impl UnityProject {
    pub fn locked_packages(&self) -> impl Iterator<Item = LockedDependencyInfo<'_>> {
        self.manifest.all_locked()
    }

    pub fn dependencies(&self) -> impl Iterator<Item = &str> {
        self.manifest.dependencies().map(|(name, _)| name)
    }

    pub fn get_locked(&self, name: &str) -> Option<LockedDependencyInfo<'_>> {
        self.manifest.get_locked(name)
    }

    pub fn is_locked(&self, name: &str) -> bool {
        self.manifest.get_locked(name).is_some()
    }

    pub fn all_packages(&self) -> impl Iterator<Item = LockedDependencyInfo<'_>> {
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

    pub fn unity_version(&self) -> UnityVersion {
        self.unity_version
    }

    pub fn unity_revision(&self) -> Option<&str> {
        self.unity_revision.as_deref()
    }

    pub fn has_upm_package(&self, name: &str) -> bool {
        self.upm_manifest.get_dependency(name).is_some()
    }

    /// Adds dependency without actually adding package.
    /// This only modifies manifest
    pub fn add_dependency_raw(&mut self, name: &str, version: DependencyRange) {
        self.manifest.add_dependency(name, version)
    }
}

impl UnityProject {
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

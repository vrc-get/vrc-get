//! The vpm client library.
//!
//! TODO: documentation

#![deny(unsafe_code)]

use std::fmt::Display;
use std::path::Path;

use indexmap::IndexMap;

use crate::package_json::PartialUnityVersion;
use version::{ReleaseType, UnityVersion, Version, VersionRange};

pub mod environment;
pub mod io;
pub mod package_json;
pub mod repository;
mod structs;
mod traits;
pub mod unity_project;
mod utils;
pub mod version;
mod version_selector;

pub use environment::Environment;
pub use unity_project::UnityProject;
pub use version_selector::VersionSelector;

use crate::repository::local::LocalCachedRepository;
pub use traits::HttpClient;
pub use traits::PackageCollection;
pub use traits::RemotePackageDownloader;

pub use package_json::PackageJson;
pub use structs::setting::UserRepoSetting;

#[derive(Debug, Copy, Clone)]
pub struct PackageInfo<'a> {
    inner: PackageInfoInner<'a>,
}

#[derive(Debug, Copy, Clone)]
enum PackageInfoInner<'a> {
    Remote(&'a PackageJson, &'a LocalCachedRepository),
    Local(&'a PackageJson, &'a Path),
}

impl<'a> PackageInfo<'a> {
    pub fn package_json(self) -> &'a PackageJson {
        // this match will be removed in the optimized code because package.json is exists at first
        match self.inner {
            PackageInfoInner::Remote(pkg, _) => pkg,
            PackageInfoInner::Local(pkg, _) => pkg,
        }
    }

    pub fn remote(json: &'a PackageJson, repo: &'a LocalCachedRepository) -> Self {
        Self {
            inner: PackageInfoInner::Remote(json, repo),
        }
    }

    pub fn local(json: &'a PackageJson, path: &'a Path) -> Self {
        Self {
            inner: PackageInfoInner::Local(json, path),
        }
    }

    #[allow(unused)]
    pub fn is_remote(self) -> bool {
        matches!(self.inner, PackageInfoInner::Remote(_, _))
    }

    #[allow(unused)]
    pub fn is_local(self) -> bool {
        matches!(self.inner, PackageInfoInner::Local(_, _))
    }

    pub fn name(self) -> &'a str {
        self.package_json().name()
    }

    pub fn version(self) -> &'a Version {
        self.package_json().version()
    }

    pub fn vpm_dependencies(self) -> &'a IndexMap<Box<str>, VersionRange> {
        self.package_json().vpm_dependencies()
    }

    pub fn legacy_packages(self) -> &'a [Box<str>] {
        self.package_json().legacy_packages()
    }

    pub fn unity(self) -> Option<&'a PartialUnityVersion> {
        self.package_json().unity()
    }

    pub fn is_yanked(self) -> bool {
        self.package_json().is_yanked()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Unknown,
    LegacySdk2,
    LegacyWorlds,
    LegacyAvatars,
    UpmWorlds,
    UpmAvatars,
    UpmStarter,
    Worlds,
    Avatars,
    VpmStarter,
}

impl Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => f.write_str("Unknown"),
            Self::LegacySdk2 => f.write_str("Legacy SDK2"),
            Self::LegacyWorlds => f.write_str("Legacy Worlds"),
            Self::LegacyAvatars => f.write_str("Legacy Avatars"),
            Self::UpmWorlds => f.write_str("UPM Worlds"),
            Self::UpmAvatars => f.write_str("UPM Avatars"),
            Self::UpmStarter => f.write_str("UPM Starter"),
            Self::Worlds => f.write_str("Worlds"),
            Self::Avatars => f.write_str("Avatars"),
            Self::VpmStarter => f.write_str("VPM Starter"),
        }
    }
}

#[cfg(feature = "vrc-get-litedb")]
impl From<vrc_get_litedb::ProjectType> for ProjectType {
    fn from(value: vrc_get_litedb::ProjectType) -> Self {
        match value {
            vrc_get_litedb::ProjectType::LEGACY_SDK2 => Self::LegacySdk2,
            vrc_get_litedb::ProjectType::LEGACY_WORLDS => Self::LegacyWorlds,
            vrc_get_litedb::ProjectType::LEGACY_AVATARS => Self::LegacyAvatars,
            vrc_get_litedb::ProjectType::UPM_WORLDS => Self::UpmWorlds,
            vrc_get_litedb::ProjectType::UPM_AVATARS => Self::UpmAvatars,
            vrc_get_litedb::ProjectType::UPM_STARTER => Self::UpmStarter,
            vrc_get_litedb::ProjectType::WORLDS => Self::Worlds,
            vrc_get_litedb::ProjectType::AVATARS => Self::Avatars,
            vrc_get_litedb::ProjectType::VPM_STARTER => Self::VpmStarter,
            vrc_get_litedb::ProjectType::UNKNOWN => Self::Unknown,
            _ => Self::Unknown,
        }
    }
}

#[cfg(feature = "vrc-get-litedb")]
impl From<ProjectType> for vrc_get_litedb::ProjectType {
    fn from(value: ProjectType) -> Self {
        match value {
            ProjectType::LegacySdk2 => Self::LEGACY_SDK2,
            ProjectType::LegacyWorlds => Self::LEGACY_WORLDS,
            ProjectType::LegacyAvatars => Self::LEGACY_AVATARS,
            ProjectType::UpmWorlds => Self::UPM_WORLDS,
            ProjectType::UpmAvatars => Self::UPM_AVATARS,
            ProjectType::UpmStarter => Self::UPM_STARTER,
            ProjectType::Worlds => Self::WORLDS,
            ProjectType::Avatars => Self::AVATARS,
            ProjectType::VpmStarter => Self::VPM_STARTER,
            ProjectType::Unknown => Self::UNKNOWN,
        }
    }
}

fn unity_compatible(package: &PackageJson, unity: UnityVersion) -> bool {
    fn is_vrcsdk_for_2019(version: &Version) -> bool {
        version.major == 3 && version.minor <= 4
    }

    fn is_resolver_for_2019(version: &Version) -> bool {
        version.major == 0 && version.minor == 1 && version.patch <= 26
    }

    match package.name() {
        "com.vrchat.avatars" | "com.vrchat.worlds" | "com.vrchat.base"
            if is_vrcsdk_for_2019(package.version()) =>
        {
            // this version of VRCSDK is only for unity 2019 so for other version(s) of unity, it's not satisfied.
            unity.major() == 2019
        }
        "com.vrchat.core.vpm-resolver" if is_resolver_for_2019(package.version()) => {
            // this version of Resolver is only for unity 2019 so for other version(s) of unity, it's not satisfied.
            unity.major() == 2019
        }
        _ => {
            // otherwice, check based on package info

            if let Some(min_unity) = package.unity() {
                unity
                    >= UnityVersion::new(
                        min_unity.major(),
                        min_unity.minor(),
                        0,
                        ReleaseType::Alpha,
                        0,
                    )
            } else {
                // if there are no info, satisfies for all unity versions
                true
            }
        }
    }
}

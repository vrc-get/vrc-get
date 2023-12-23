//! The vpm client library.
//!
//! TODO: documentation

#![forbid(unsafe_code)]

use std::path::Path;

use indexmap::IndexMap;
use serde_json::{Map, Value};

use structs::package::PartialUnityVersion;
use version::{ReleaseType, UnityVersion, Version, VersionRange};

pub mod environment;
pub mod repository;
mod structs;
mod traits;
pub mod unity_project;
mod utils;
pub mod version;
mod version_selector;

type JsonMap = Map<String, Value>;

pub use environment::Environment;
pub use unity_project::UnityProject;
pub use version_selector::VersionSelector;

use crate::repository::local::LocalCachedRepository;
pub use traits::HttpClient;
pub use traits::PackageCollection;
pub use traits::RemotePackageDownloader;

pub use structs::package::PackageJson;
pub use structs::setting::UserRepoSetting;

#[derive(Copy, Clone)]
pub struct PackageInfo<'a> {
    inner: PackageInfoInner<'a>,
}

#[derive(Copy, Clone)]
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

    pub(crate) fn remote(json: &'a PackageJson, repo: &'a LocalCachedRepository) -> Self {
        Self {
            inner: PackageInfoInner::Remote(json, repo),
        }
    }

    pub(crate) fn local(json: &'a PackageJson, path: &'a Path) -> Self {
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

    pub fn vpm_dependencies(self) -> &'a IndexMap<String, VersionRange> {
        self.package_json().vpm_dependencies()
    }

    pub fn legacy_packages(self) -> &'a [String] {
        self.package_json().legacy_packages()
    }

    pub fn unity(self) -> Option<&'a PartialUnityVersion> {
        self.package_json().unity()
    }

    #[cfg(feature = "experimental-yank")]
    pub fn is_yanked(self) -> bool {
        self.package_json().is_yanked()
    }
}

fn unity_compatible(package: &PackageInfo, unity: UnityVersion) -> bool {
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
                unity >= UnityVersion::new(min_unity.0, min_unity.1, 0, ReleaseType::Alpha, 0)
            } else {
                // if there are no info, satisfies for all unity versions
                true
            }
        }
    }
}

use crate::version::{DependencyRange, Version, VersionRange};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

pub mod manifest {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct VpmDependency {
        pub version: DependencyRange,
    }

    impl VpmDependency {
        pub fn new(version: Version) -> Self {
            Self {
                version: DependencyRange::version(version),
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct VpmLockedDependency {
        pub version: Version,
        #[serde(default, skip_serializing_if = "indexmap::IndexMap::is_empty")]
        pub dependencies: IndexMap<String, VersionRange>,
    }

    impl VpmLockedDependency {
        pub fn new(
            version: Version,
            dependencies: IndexMap<String, VersionRange>,
        ) -> VpmLockedDependency {
            Self {
                version,
                dependencies,
            }
        }
    }
}

pub mod package {
    use super::*;
    use crate::utils::is_truthy;
    use url::Url;
    #[derive(Deserialize, Debug, Clone)]
    pub struct PackageJson {
        pub name: String,
        #[serde(rename = "displayName")]
        #[serde(default)]
        pub display_name: Option<String>,
        pub description: Option<String>,
        pub version: Version,
        #[serde(rename = "vpmDependencies")]
        #[serde(default)]
        pub vpm_dependencies: IndexMap<String, VersionRange>,
        #[serde(default)]
        pub url: Option<Url>,

        pub unity: Option<PartialUnityVersion>,

        #[serde(rename = "legacyFolders")]
        #[serde(default)]
        pub legacy_folders: HashMap<String, Option<String>>,
        #[serde(rename = "legacyFiles")]
        #[serde(default)]
        pub legacy_files: HashMap<String, Option<String>>,
        #[serde(rename = "legacyPackages")]
        #[serde(default)]
        pub legacy_packages: Vec<String>,

        #[serde(default)]
        pub yanked: Option<Value>,
    }

    impl PackageJson {
        pub fn is_yanked(&self) -> bool {
            is_truthy(self.yanked.as_ref())
        }
    }

    #[derive(Debug, Clone)]
    pub struct PartialUnityVersion(pub u16, pub u8);

    impl<'de> Deserialize<'de> for PartialUnityVersion {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::de::Deserializer<'de>,
        {
            use serde::de::Error;
            let s = String::deserialize(deserializer)?;
            if let Some((maj, min)) = s.split_once('.') {
                let major = maj.trim().parse::<u16>().map_err(Error::custom)?;
                let minor = min.trim().parse::<u8>().map_err(Error::custom)?;
                Ok(Self(major, minor))
            } else {
                let major = s.trim().parse::<u16>().map_err(Error::custom)?;
                Ok(Self(major, 0))
            }
        }
    }
}

pub mod setting {
    use super::*;
    use url::Url;
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct UserRepoSetting {
        #[serde(rename = "localPath")]
        pub local_path: PathBuf,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        // must be non-relative url.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub url: Option<Url>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub id: Option<String>,
        #[serde(default)]
        pub headers: IndexMap<String, String>,
    }

    impl UserRepoSetting {
        pub fn new(
            local_path: PathBuf,
            name: Option<String>,
            url: Option<Url>,
            id: Option<String>,
        ) -> Self {
            Self {
                local_path,
                name,
                id: id.or(url.as_ref().map(Url::to_string)),
                url,
                headers: IndexMap::new(),
            }
        }
    }
}

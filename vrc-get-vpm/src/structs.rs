use crate::version::{DependencyRange, Version, VersionRange};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
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
    use url::Url;
    #[derive(Deserialize, Debug, Clone)]
    pub struct PackageJson {
        name: String,
        #[serde(rename = "displayName")]
        #[serde(default)]
        display_name: Option<String>,
        description: Option<String>,
        version: Version,
        #[serde(rename = "vpmDependencies")]
        #[serde(default)]
        vpm_dependencies: IndexMap<String, VersionRange>,
        #[serde(default)]
        url: Option<Url>,

        unity: Option<PartialUnityVersion>,

        #[serde(rename = "legacyFolders")]
        #[serde(default)]
        legacy_folders: HashMap<String, Option<String>>,
        #[serde(rename = "legacyFiles")]
        #[serde(default)]
        legacy_files: HashMap<String, Option<String>>,
        #[serde(rename = "legacyPackages")]
        #[serde(default)]
        legacy_packages: Vec<String>,

        #[cfg(feature = "experimental-yank")]
        #[serde(default)]
        yanked: Option<serde_json::Value>,
    }

    impl PackageJson {
        pub fn name(&self) -> &str {
            &self.name
        }

        pub fn version(&self) -> &Version {
            &self.version
        }

        pub fn vpm_dependencies(&self) -> &IndexMap<String, VersionRange> {
            &self.vpm_dependencies
        }

        pub fn legacy_folders(&self) -> &HashMap<String, Option<String>> {
            &self.legacy_folders
        }

        pub fn legacy_files(&self) -> &HashMap<String, Option<String>> {
            &self.legacy_files
        }

        pub fn legacy_packages(&self) -> &[String] {
            self.legacy_packages.as_slice()
        }

        pub fn display_name(&self) -> Option<&str> {
            self.display_name.as_deref()
        }

        pub fn description(&self) -> Option<&str> {
            self.description.as_deref()
        }

        pub fn url(&self) -> Option<&Url> {
            self.url.as_ref()
        }

        pub fn unity(&self) -> Option<&PartialUnityVersion> {
            self.unity.as_ref()
        }

        #[cfg(feature = "experimental-yank")]
        pub fn is_yanked(&self) -> bool {
            crate::utils::is_truthy(self.yanked.as_ref())
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
    use std::path::Path;
    use super::*;
    use url::Url;
    use crate::environment::RepoSource;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct UserRepoSetting {
        #[serde(rename = "localPath")]
        local_path: PathBuf,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        // must be non-relative url.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<Url>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub(crate) id: Option<String>,
        #[serde(default)]
        headers: IndexMap<String, String>,
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

        pub fn local_path(&self) -> &Path {
            &self.local_path
        }

        pub fn name(&self) -> Option<&str> {
            self.name.as_deref()
        }

        pub fn url(&self) -> Option<&Url> {
            self.url.as_ref()
        }

        pub fn id(&self) -> Option<&str> {
            self.id.as_deref()
        }

        pub fn headers(&self) -> &IndexMap<String, String> {
            &self.headers
        }
    }

    impl RepoSource for UserRepoSetting {
        fn cache_path(&self) -> &Path {
            self.local_path()
        }

        fn headers(&self) -> &IndexMap<String, String> {
            self.headers()
        }

        fn url(&self) -> Option<&Url> {
            self.url()
        }
    }
}

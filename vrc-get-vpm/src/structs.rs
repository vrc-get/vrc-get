use crate::version::{Version, VersionRange};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod package {
    use super::*;
    use url::Url;
    #[derive(Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct PackageJson {
        name: Box<str>,
        #[serde(default)]
        display_name: Option<Box<str>>,
        description: Option<Box<str>>,
        version: Version,
        #[serde(default)]
        vpm_dependencies: IndexMap<Box<str>, VersionRange>,
        #[serde(default)]
        url: Option<Url>,

        unity: Option<PartialUnityVersion>,

        #[serde(default)]
        legacy_folders: HashMap<Box<str>, Option<Box<str>>>,
        #[serde(default)]
        legacy_files: HashMap<Box<str>, Option<Box<str>>>,
        #[serde(default)]
        legacy_packages: Vec<Box<str>>,

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

        pub fn vpm_dependencies(&self) -> &IndexMap<Box<str>, VersionRange> {
            &self.vpm_dependencies
        }

        pub fn legacy_folders(&self) -> &HashMap<Box<str>, Option<Box<str>>> {
            &self.legacy_folders
        }

        pub fn legacy_files(&self) -> &HashMap<Box<str>, Option<Box<str>>> {
            &self.legacy_files
        }

        pub fn legacy_packages(&self) -> &[Box<str>] {
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
    use super::*;
    use crate::environment::RepoSource;
    use std::path::Path;
    use url::Url;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct UserRepoSetting {
        local_path: Box<Path>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<Box<str>>,
        // must be non-relative url.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<Url>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub(crate) id: Option<Box<str>>,
        #[serde(default)]
        headers: IndexMap<Box<str>, Box<str>>,
    }

    impl UserRepoSetting {
        pub fn new(
            local_path: Box<Path>,
            name: Option<Box<str>>,
            url: Option<Url>,
            id: Option<Box<str>>,
        ) -> Self {
            Self {
                local_path,
                name,
                id: id.or(url.as_ref().map(Url::to_string).map(Into::into)),
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

        pub fn headers(&self) -> &IndexMap<Box<str>, Box<str>> {
            &self.headers
        }

        pub(crate) fn to_source(&self) -> RepoSource {
            RepoSource::new(&self.local_path, &self.headers, self.url.as_ref())
        }
    }
}

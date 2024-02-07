use crate::version::{Version, VersionRange};
use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::HashMap;
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
pub struct PartialUnityVersion(u16, u8);

impl PartialUnityVersion {
    pub fn major(&self) -> u16 {
        self.0
    }

    pub fn minor(&self) -> u8 {
        self.1
    }
}

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

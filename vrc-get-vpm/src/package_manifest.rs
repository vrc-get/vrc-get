use crate::version::{Version, VersionRange};
use indexmap::IndexMap;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt::Formatter;
use url::Url;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PackageManifest {
    name: Box<str>,
    #[serde(default)]
    display_name: Option<Box<str>>,
    description: Option<Box<str>>,
    version: Version,
    #[serde(default)]
    vpm_dependencies: IndexMap<Box<str>, VersionRange>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    #[serde(rename = "zipSHA256")]
    zip_sha_256: Option<Box<str>>,

    unity: Option<PartialUnityVersion>,

    #[serde(default)]
    legacy_folders: HashMap<Box<str>, Option<Box<str>>>,
    #[serde(default)]
    legacy_files: HashMap<Box<str>, Option<Box<str>>>,
    #[serde(default)]
    legacy_packages: Vec<Box<str>>,

    #[serde(default)]
    changelog_url: Option<Url>,

    #[serde(rename = "vrc-get")]
    #[serde(default)]
    vrc_get: VrcGetMeta,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VrcGetMeta {
    #[serde(default)]
    yanked: YankState,
    /// aliases for `vrc-get i --name <name> <version>` command.
    #[serde(default)]
    aliases: Vec<Box<str>>,
}

/// Constructing PackageJson. Especially for testing.
impl PackageManifest {
    pub fn new(name: impl Into<Box<str>>, version: Version) -> Self {
        Self {
            name: name.into(),
            display_name: None,
            description: None,
            version,
            vpm_dependencies: IndexMap::new(),
            url: None,
            unity: None,
            legacy_folders: HashMap::new(),
            legacy_files: HashMap::new(),
            legacy_packages: Vec::new(),
            vrc_get: VrcGetMeta::default(),
            zip_sha_256: None,
            changelog_url: None,
        }
    }

    pub fn add_vpm_dependency(mut self, name: impl Into<Box<str>>, range: &str) -> Self {
        self.vpm_dependencies
            .insert(name.into(), range.parse().unwrap());
        self
    }

    pub fn add_legacy_package(mut self, name: impl Into<Box<str>>) -> Self {
        self.legacy_packages.push(name.into());
        self
    }

    pub fn add_legacy_folder(
        mut self,
        path: impl Into<Box<str>>,
        guid: impl Into<Box<str>>,
    ) -> Self {
        self.legacy_folders.insert(path.into(), Some(guid.into()));
        self
    }

    pub fn add_legacy_file(mut self, path: impl Into<Box<str>>, guid: impl Into<Box<str>>) -> Self {
        self.legacy_files.insert(path.into(), Some(guid.into()));
        self
    }
}

impl PackageManifest {
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

    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    pub fn zip_sha_256(&self) -> Option<&str> {
        self.zip_sha_256.as_deref()
    }

    pub fn changelog_url(&self) -> Option<&Url> {
        self.changelog_url.as_ref()
    }

    pub fn unity(&self) -> Option<&PartialUnityVersion> {
        self.unity.as_ref()
    }

    pub fn is_yanked(&self) -> bool {
        self.vrc_get.yanked.is_yanked()
    }

    pub fn aliases(&self) -> &[Box<str>] {
        self.vrc_get.aliases.as_slice()
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
        D: Deserializer<'de>,
    {
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

#[derive(Debug, Clone, Default)]
enum YankState {
    #[default]
    NotYanked,
    NoReason,
    Reason(Box<str>),
}

impl YankState {
    pub fn is_yanked(&self) -> bool {
        match self {
            YankState::NotYanked => false,
            YankState::NoReason => true,
            YankState::Reason(_) => true,
        }
    }

    #[allow(dead_code)]
    pub fn reason(&self) -> Option<&str> {
        match self {
            YankState::Reason(s) => Some(s),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for YankState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VisitorImpl;
        impl<'de> serde::de::Visitor<'de> for VisitorImpl {
            type Value = YankState;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a boolean or a string")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if v {
                    Ok(YankState::NoReason)
                } else {
                    Ok(YankState::NotYanked)
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(YankState::Reason(v.into()))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(YankState::Reason(v.into()))
            }
        }

        deserializer.deserialize_any(VisitorImpl)
    }
}

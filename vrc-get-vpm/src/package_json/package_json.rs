use indexmap::IndexMap;
use serde::{Deserialize, Deserializer};
use std::fmt::Debug;
use url::Url;

use crate::utils::DedupForwarder;
use crate::version::{Version, VersionRange};
use crate::PartialUnityVersion;

#[derive(Clone)]
pub struct PackageJson {
    inner: inner::PackageJson,
}

impl Debug for PackageJson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

mod inner {
    use super::super::package_manifest::YankState;
    use crate::version::{Version, VersionRange};
    use crate::PartialUnityVersion;
    use indexmap::IndexMap;
    use serde::{Deserialize, Deserializer};
    use url::Url;

    fn default_if_err<'de, D, T>(de: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de> + Default,
    {
        match T::deserialize(de) {
            Ok(v) => Ok(v),
            Err(_) => Ok(T::default()),
        }
    }

    // Note: please keep in sync with package_manifest
    #[derive(Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct PackageJson {
        pub(super) name: Box<str>,
        pub(super) version: Version,

        #[serde(default, deserialize_with = "default_if_err")]
        pub(super) display_name: Option<Box<str>>,
        #[serde(default, deserialize_with = "default_if_err")]
        pub(super) description: Option<Box<str>>,
        #[serde(default, deserialize_with = "default_if_err")]
        pub(super) vpm_dependencies: IndexMap<Box<str>, VersionRange>,

        #[serde(default, deserialize_with = "default_if_err")]
        pub(super) unity: Option<PartialUnityVersion>,

        #[serde(default, deserialize_with = "default_if_err")]
        pub(super) legacy_packages: Vec<Box<str>>,

        #[serde(default, deserialize_with = "default_if_err")]
        pub(super) changelog_url: Option<Url>,

        #[serde(rename = "vrc-get")]
        #[serde(default, deserialize_with = "default_if_err")]
        pub(super) vrc_get: VrcGetMeta,
    }

    // Note: please keep in sync with package_manifest
    #[derive(Deserialize, Debug, Clone, Default)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct VrcGetMeta {
        #[serde(default, deserialize_with = "default_if_err")]
        pub(super) yanked: YankState,
        /// aliases for `vrc-get i --name <name> <version>` command.
        #[serde(default, deserialize_with = "default_if_err")]
        pub(super) aliases: Vec<Box<str>>,
    }
}

impl<'de> Deserialize<'de> for PackageJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner = inner::PackageJson::deserialize(DedupForwarder::new(deserializer))?;
        Ok(Self { inner })
    }
}

impl PackageJson {
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    pub fn version(&self) -> &Version {
        &self.inner.version
    }

    pub fn vpm_dependencies(&self) -> &IndexMap<Box<str>, VersionRange> {
        &self.inner.vpm_dependencies
    }

    pub fn legacy_packages(&self) -> &[Box<str>] {
        self.inner.legacy_packages.as_slice()
    }

    pub fn display_name(&self) -> Option<&str> {
        self.inner.display_name.as_deref()
    }

    pub fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    pub fn changelog_url(&self) -> Option<&Url> {
        self.inner.changelog_url.as_ref()
    }

    pub fn unity(&self) -> Option<&PartialUnityVersion> {
        self.inner.unity.as_ref()
    }

    pub fn is_yanked(&self) -> bool {
        self.inner.vrc_get.yanked.is_yanked()
    }

    pub fn aliases(&self) -> &[Box<str>] {
        self.inner.vrc_get.aliases.as_slice()
    }
}

impl_package_json_like!(PackageJson);

#[test]
fn deserialize_partially_bad() {
    let json = r#"{
        "name": "vrc-get-vpm",
        "version": "0.1.0",
        "vpmDependencies": {
            "vrc-get": ">=0.1.0"
        },
        "comment": "Thre following is duplicated key url",
        "legacyPackages": ["vrc-get"],
        "legacyPackages": ["vrc-2"],
        "comment": "Thre following is invalid url",
        "changelog_url": "",
        "vrc-get": {
            "yanked": false,
            "aliases": ["vpm"]
        }
    }"#;
    let package_json: PackageJson = serde_json::from_str(json).unwrap();
    assert_eq!(package_json.name(), "vrc-get-vpm");
    assert_eq!(package_json.version(), &Version::new(0, 1, 0));
    assert_eq!(package_json.vpm_dependencies(), &{
        let mut map = IndexMap::new();
        map.insert(
            "vrc-get".into(),
            VersionRange::same_or_later(Version::new(0, 1, 0)),
        );
        map
    });
    assert_eq!(package_json.legacy_packages(), &["vrc-get".into()]);
    assert!(!package_json.is_yanked());
    assert_eq!(package_json.aliases(), &["vpm".into()]);
    assert_eq!(package_json.changelog_url(), None);
}

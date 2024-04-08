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

impl_package_json!(impl PackageJson = |value| value.inner);

impl Debug for PackageJson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
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

mod inner {
    use super::super::YankState;
    use crate::version::{Version, VersionRange};
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

    package_json_struct! {
        #[derive(Deserialize, Debug, Clone)]
        #[serde(rename_all = "camelCase")]
        pub(super) struct PackageJson {
            pub(super) optional: #[serde(default, deserialize_with = "default_if_err")];
            pub(super) required;
            pub(super) vrc_get: #[serde(rename = "vrc-get")];
        }
        #[derive(Deserialize, Debug, Clone, Default)]
        #[serde(rename_all = "camelCase")]
        pub(super) struct VrcGetMeta {
            pub(super) optional: #[serde(default, deserialize_with = "default_if_err")];
        }
    }
}

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

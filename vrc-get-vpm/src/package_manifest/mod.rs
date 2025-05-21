mod partial_unity_version;
mod yank_state;

use crate::utils::DedupForwarder;
use crate::version::{Version, VersionRange};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use url::Url;

use crate::package_manifest::yank_state::YankState;
pub use partial_unity_version::PartialUnityVersion;

macro_rules! initialize_from_package_json_like {
    ($source: expr) => {
        PackageManifest {
            name: $source.name,
            version: $source.version,
            display_name: $source.display_name,
            description: $source.description,
            unity: $source.unity,
            url: $source.url,
            zip_sha_256: $source.zip_sha_256,
            vpm_dependencies: $source.vpm_dependencies,
            legacy_folders: $source.legacy_folders,
            legacy_files: $source.legacy_files,
            legacy_packages: $source.legacy_packages,
            headers: $source.headers,
            changelog_url: $source.changelog_url,
            documentation_url: $source.documentation_url,
            vrc_get: VrcGetMeta {
                yanked: $source.vrc_get.yanked,
                aliases: $source.vrc_get.aliases,
            },
        }
    };
}

macro_rules! package_json_struct {
    {
        $(#[$meta:meta])*
        $vis:vis struct $name: ident {
            $optional_vis:vis optional$(: #[$optional: meta])?;
            $required_vis:vis required$(: #[$required: meta])?;
        }
        $(#[$vr_get_meta:meta])*
        $vrc_get_struct_vis:vis struct $vrc_get_meta_name:ident {
            $vrc_get_optional_vis:vis optional$(: #[$vrc_get_optional: meta])?;
        }
    } => {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        $(#[$meta])*
        $vis struct $name {
            $(#[$required])?
            $required_vis name: Box<str>,
            $(#[$required])?
            $required_vis version: Version,

            $(#[$optional])?
            $optional_vis display_name: Option<Box<str>>,
            $(#[$optional])?
            $optional_vis description: Option<Box<str>>,
            $(#[$optional])?
            $optional_vis unity: Option<PartialUnityVersion>,

            $(#[$optional])?
            $optional_vis url: Option<Url>,
            $(#[$optional])?
            #[serde(rename = "zipSHA256")]
            $optional_vis zip_sha_256: Option<Box<str>>,

            $(#[$optional])?
            $optional_vis vpm_dependencies: IndexMap<Box<str>, VersionRange>,

            $(#[$optional])?
            $optional_vis legacy_folders: HashMap<Box<str>, Option<Box<str>>>,
            $(#[$optional])?
            $optional_vis legacy_files: HashMap<Box<str>, Option<Box<str>>>,
            $(#[$optional])?
            $optional_vis legacy_packages: Vec<Box<str>>,

            $(#[$optional])?
            $optional_vis headers: indexmap::IndexMap<Box<str>, Box<str>>,

            $(#[$optional])?
            $optional_vis changelog_url: Option<Url>,
            $(#[$optional])?
            $optional_vis documentation_url: Option<Url>,

            $(#[$optional])?
            #[serde(rename = "vrc-get")]
            $optional_vis vrc_get: $vrc_get_meta_name,
        }

        // Note: please keep in sync with package_manifest
        $(#[$vr_get_meta])*
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        $vrc_get_struct_vis struct $vrc_get_meta_name {
            $(#[$vrc_get_optional])?
            $vrc_get_optional_vis yanked: YankState,
            /// aliases for `vrc-get i --name <name> <version>` command.
            $(#[$vrc_get_optional])?
            $vrc_get_optional_vis aliases: Vec<Box<str>>,
        }
    };
}

package_json_struct! {
    #[derive(Debug, Clone)]
    pub struct PackageManifest {
        optional: #[serde(default)];
        required;
    }
    #[derive(Debug, Clone, Default)]
    pub(super) struct VrcGetMeta {
        optional: #[serde(default)];
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
    pub fn headers(&self) -> &IndexMap<Box<str>, Box<str>> {
        &self.headers
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
    pub fn zip_sha_256(&self) -> Option<&str> {
        self.zip_sha_256.as_deref()
    }
    pub fn changelog_url(&self) -> Option<&Url> {
        self.changelog_url.as_ref()
    }
    pub fn documentation_url(&self) -> Option<&Url> {
        self.documentation_url.as_ref()
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

/// Constructing PackageJson. Especially for testing.
impl PackageManifest {
    pub fn new(name: impl Into<Box<str>>, version: Version) -> Self {
        Self {
            name: name.into(),
            version,
            display_name: None,
            description: None,
            vpm_dependencies: IndexMap::new(),
            url: None,
            unity: None,
            legacy_folders: HashMap::new(),
            legacy_files: HashMap::new(),
            legacy_packages: Vec::new(),
            headers: IndexMap::new(),
            vrc_get: VrcGetMeta::default(),
            zip_sha_256: None,
            changelog_url: None,
            documentation_url: None,
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

pub(crate) struct LooseManifest(pub PackageManifest);

impl<'de> Deserialize<'de> for LooseManifest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
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
            pub(super) struct LooseManifest {
                pub(super) optional: #[serde(default, deserialize_with = "default_if_err")];
                pub(super) required;
            }
            #[derive(Default)]
            pub(super) struct LooseVrcGetMeta {
                pub(super) optional: #[serde(default, deserialize_with = "default_if_err")];
            }
        }

        let strict = LooseManifest::deserialize(DedupForwarder::new(deserializer))?;

        Ok(LooseManifest(initialize_from_package_json_like!(strict)))
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
        "url": "",
        "vrc-get": {
            "yanked": false,
            "aliases": ["vpm"]
        }
    }"#;
    let package_json: LooseManifest = serde_json::from_str(json).unwrap();
    let package_json = package_json.0;
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

use crate::package_json::yank_state::YankState;
use crate::version::{Version, VersionRange};
use crate::PartialUnityVersion;
use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::HashMap;
use url::Url;

package_json_struct! {
    pub struct PackageManifest {
        optional: #[serde(default)];
        required;
    }
}

impl_package_json!(impl PackageManifest = |value| value);

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

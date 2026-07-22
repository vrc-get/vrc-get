mod partial_unity_version;
mod yank_state;

use crate::utils::json::{JsonError, JsonValue};
use crate::version::{Version, VersionRange};
use indexmap::IndexMap;
use itertools::Itertools;
use std::collections::HashMap;
use std::str::FromStr;
use url::Url;

use crate::package_manifest::yank_state::YankState;
pub use partial_unity_version::PartialUnityVersion;

#[derive(Debug, Clone)]
pub struct PackageManifest {
    name: Box<str>,
    version: Version,
    display_name: Option<Box<str>>,
    description: Option<Box<str>>,
    unity: Option<PartialUnityVersion>,
    url: Option<Url>,
    zip_sha_256: Option<Box<str>>,
    vpm_dependencies: IndexMap<Box<str>, VersionRange>,
    legacy_folders: HashMap<Box<str>, Option<Box<str>>>,
    legacy_files: HashMap<Box<str>, Option<Box<str>>>,
    legacy_packages: Vec<Box<str>>,
    headers: IndexMap<Box<str>, Box<str>>,
    changelog_url: Option<Url>,
    documentation_url: Option<Url>,
    keywords: Vec<Box<str>>,
    vrc_get: VrcGetMeta,
}

#[derive(Debug, Clone, Default)]
pub(super) struct VrcGetMeta {
    yanked: YankState,
    aliases: Vec<Box<str>>,
}

impl PackageManifest {
    pub(crate) fn from_json_value(value: JsonValue) -> Result<Self, JsonError> {
        Self::from_json_value_impl::<ErrorAsError>(value)
    }

    pub(crate) fn from_loose_json_value(value: JsonValue) -> Result<Self, JsonError> {
        Self::from_json_value_impl::<ErrorAsDefault>(value)
    }

    fn from_json_value_impl<H: ErrorHandler>(value: JsonValue) -> Result<Self, JsonError> {
        let mut object = value.into_object()?;
        Ok(Self {
            name: (object.get_req("name"))?.into_string()?.into(),
            version: (object.get_req("version"))?.parse_req(|x| x.parse())?,
            display_name: H::h((object.get_opt("displayName")).try_map(JsonValue::into_string))?
                .map(Into::into),
            description: H::h((object.get_opt("description")).try_map(JsonValue::into_string))?
                .map(Into::into),
            unity: H::h(object.get_opt("unity").parse_opt(|x| x.parse()))?,
            url: H::h(object.get_opt("url").parse_opt(parse_optional_url))?.flatten(),
            zip_sha_256: H::h(object.get_opt("zipSHA256").try_map(JsonValue::into_string))?
                .map(Into::into),
            vpm_dependencies: H::h((object.get_opt("vpmDependencies")).try_map(|value| {
                let object = value.into_object()?;
                object
                    .into_iter()
                    .map(|(key, value)| Ok((key.into(), value.parse_req(|x| x.parse())?)))
                    .collect::<Result<_, _>>()
            }))?
            .unwrap_or_default(),
            legacy_folders: H::h((object.get_opt("legacyFolders")).try_map(|value| {
                (value.into_object()?)
                    .into_iter()
                    .map(|(key, value)| {
                        Ok((
                            key.into(),
                            value.try_map(JsonValue::into_string)?.map(Into::into),
                        ))
                    })
                    .collect::<Result<_, _>>()
            }))?
            .unwrap_or_default(),
            legacy_files: H::h((object.get_opt("legacyFiles")).try_map(|value| {
                (value.into_object()?)
                    .into_iter()
                    .map(|(key, value)| {
                        Ok((
                            key.into(),
                            value.try_map(JsonValue::into_string)?.map(Into::into),
                        ))
                    })
                    .collect()
            }))?
            .unwrap_or_default(),
            legacy_packages: H::h((object.get_opt("legacyPackages")).try_map(|value| {
                let array = value.into_array()?;
                array
                    .into_iter()
                    .map(|value| value.into_string().map(Into::into))
                    .collect::<Result<_, _>>()
            }))?
            .unwrap_or_default(),
            headers: H::h((object.get_opt("headers")).try_map(|value| {
                let object = value.into_object()?;
                object
                    .into_iter()
                    .map(|(key, value)| Ok((key.into(), value.into_string()?.into())))
                    .collect::<Result<_, _>>()
            }))?
            .unwrap_or_default(),
            changelog_url: H::h((object.get_opt("changelogUrl")).parse_opt(parse_optional_url))?
                .flatten(),
            documentation_url: H::h(
                (object.get_opt("documentationUrl")).parse_opt(parse_optional_url),
            )?
            .flatten(),
            keywords: H::h((object.get_opt("legacyPackages")).try_map(|value| {
                let array = value.into_array()?;
                array
                    .into_iter()
                    .map(|value| value.into_string().map(Into::into))
                    .collect::<Result<_, _>>()
            }))?
            .unwrap_or_default(),
            vrc_get: H::h((object.get_opt("vrc-get")).try_map(VrcGetMeta::from_json_value))?
                .unwrap_or_default(),
        })
    }
}

trait ErrorHandler {
    fn h<T: Default>(v: Result<T, JsonError>) -> Result<T, JsonError>;
}

struct ErrorAsError;

impl ErrorHandler for ErrorAsError {
    fn h<T: Default>(v: Result<T, JsonError>) -> Result<T, JsonError> {
        v
    }
}

struct ErrorAsDefault;

impl ErrorHandler for ErrorAsDefault {
    fn h<T: Default>(v: Result<T, JsonError>) -> Result<T, JsonError> {
        Ok(v.unwrap_or_default())
    }
}

fn parse_optional_url(url: String) -> Result<Option<Url>, String> {
    if url.trim().is_empty() {
        return Ok(None);
    }
    Url::parse(&url).map(Some).map_err(|err| err.to_string())
}

impl VrcGetMeta {
    fn from_json_value(value: JsonValue) -> Result<VrcGetMeta, JsonError> {
        let object = value.into_object()?;

        Ok(VrcGetMeta {
            yanked: (object.get_opt("yanked").try_map(YankState::from_json))?.unwrap_or_default(),
            aliases: (object.get_opt("aliases"))
                .try_map(|value| {
                    let array = value.into_array()?;
                    array
                        .into_iter()
                        .map(|value| value.into_string())
                        .map_ok(Into::into)
                        .collect::<Result<_, _>>()
                })?
                .unwrap_or_default(),
        })
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
    // TODO: deprecate aliases on next minor release
    pub fn aliases(&self) -> &[Box<str>] {
        self.vrc_get.aliases.as_slice()
    }
    pub fn keywords(&self) -> &[Box<str>] {
        self.keywords.as_slice()
    }
}

/// Constructing PackageJson. Especially for testing.
impl PackageManifest {
    #[cfg(not(r2cs))]
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
            keywords: Vec::new(),
        }
    }

    #[cfg(not(r2cs))]
    pub fn add_vpm_dependency(mut self, name: impl Into<Box<str>>, range: &str) -> Self {
        self.vpm_dependencies
            .insert(name.into(), VersionRange::from_str(range).unwrap());
        self
    }

    #[cfg(not(r2cs))]
    pub fn add_legacy_package(mut self, name: impl Into<Box<str>>) -> Self {
        self.legacy_packages.push(name.into());
        self
    }

    #[cfg(not(r2cs))]
    pub fn add_legacy_folder(
        mut self,
        path: impl Into<Box<str>>,
        guid: impl Into<Box<str>>,
    ) -> Self {
        self.legacy_folders.insert(path.into(), Some(guid.into()));
        self
    }

    #[cfg(not(r2cs))]
    pub fn add_legacy_file(mut self, path: impl Into<Box<str>>, guid: impl Into<Box<str>>) -> Self {
        self.legacy_files.insert(path.into(), Some(guid.into()));
        self
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
    let package_json = crate::utils::json::parse_json_file(
        json.as_bytes(),
        "test",
        PackageManifest::from_loose_json_value,
    )
    .unwrap();
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
    assert_eq!(package_json.legacy_packages(), &["vrc-2".into()]);
    assert!(!package_json.is_yanked());
    assert_eq!(package_json.aliases(), &["vpm".into()]);
    assert_eq!(package_json.changelog_url(), None);
}

#[test]
fn deserialize_null_on_dependencies() {
    let json = r##"{
      "name": "com.kibalab.materialmerger",
      "displayName": "Material Merger",
      "description": "Unity Editor tool that merges multiple materials/textures into an atlas-based workflow. specifically designed for VRChat world/avatar optimization.",
      "version": "0.1.0",
      "unity": "2022.3",
      "url": "https://github.com/kibalab/material-merger/releases/download/0.1.0/com.kibalab.materialmerger-0.1.0.zip",
      "author": {
        "name": "KIBA",
        "email": "root@kiba.red",
        "url": "https://vpm.kiba.red"
      },
      "dependencies": null,
      "vpmDependencies": null,
      "samples": null,
      "zipSHA256": "0e201b9a1ed9f0e3a9c16b8f765605e8aa0c9aebf9a315c04bc67f6ebe2485f8"
    }"##;
    let package_json = crate::utils::json::parse_json_file(
        json.as_bytes(),
        "test",
        PackageManifest::from_json_value,
    )
    .unwrap();
    assert_eq!(package_json.name(), "com.kibalab.materialmerger");
    assert_eq!(package_json.version(), &Version::new(0, 1, 0));
    assert!(package_json.vpm_dependencies().is_empty());
}

#[test]
fn deserialize_empty_documentation() {
    let json = r##"{
      "name": "net.yarukizero.vrchat.shizuku",
      "displayName": "Shizuku",
      "version": "0.0.0",
      "unity": "2022.3",
      "description": "スクリプトでいい感じに定義したい",
      "vpmDependencies": {
        "nadena.dev.modular-avatar": ">=1.9.10"
      },
      "changelogUrl": " ",
      "author": {
        "name": "azumyar",
        "url": "https://github.com/azumyar"
      },
      "documentationUrl": "",
      "license": "MIT",
      "zipSHA256": "22a143ed75c429a471ffd784102d2fb577c56b010b49439b5930cbb2df820f8b",
      "url": "https://github.com/azumyar/vrchat-shizuku/releases/download/0.0.0/net.yarukizero.vrchat.shizuku-0.0.0.zip"
    }"##;
    let package_json = crate::utils::json::parse_json_file(
        json.as_bytes(),
        "test",
        PackageManifest::from_json_value,
    )
    .unwrap();
    assert_eq!(package_json.name(), "net.yarukizero.vrchat.shizuku");
    assert_eq!(package_json.version(), &Version::new(0, 0, 0));
    assert_eq!(package_json.documentation_url(), None);
    assert_eq!(package_json.changelog_url(), None);
}

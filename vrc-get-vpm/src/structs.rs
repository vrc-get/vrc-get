use crate::version::{DependencyRange, Version, VersionRange};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;

type JsonMap = Map<String, Value>;

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
        pub url: String,

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
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct UserRepoSetting {
        #[serde(rename = "localPath")]
        pub local_path: PathBuf,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        // must be non-relative url.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub id: Option<String>,
        #[serde(default)]
        pub headers: IndexMap<String, String>,
    }

    impl UserRepoSetting {
        pub fn new(
            local_path: PathBuf,
            name: Option<String>,
            url: Option<String>,
            id: Option<String>,
        ) -> Self {
            Self {
                local_path,
                name,
                id: id.or(url.clone()),
                url,
                headers: IndexMap::new(),
            }
        }
    }
}

pub mod repo_cache {
    use super::*;
    use crate::structs::package::PackageJson;
    use crate::structs::repository::{PackageVersions, Repository};

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct LocalCachedRepository {
        repo: Repository,
        #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
        headers: IndexMap<String, String>,
        #[serde(rename = "vrc-get")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub vrc_get: Option<VrcGetMeta>,
    }

    impl LocalCachedRepository {
        pub fn new(repo: Repository, headers: IndexMap<String, String>) -> Self {
            Self {
                repo,
                headers,
                vrc_get: None,
            }
        }

        pub fn headers(&self) -> &IndexMap<String, String> {
            &self.headers
        }

        pub fn repo(&self) -> &Repository {
            &self.repo
        }

        pub fn set_repo(&mut self, mut repo: Repository) {
            if let Some(id) = self.id() {
                repo.set_id_if_none(|| id.to_owned());
            }
            if let Some(url) = self.url() {
                repo.set_url_if_none(|| url.to_owned());
            }
            self.repo = repo;
        }

        pub fn url(&self) -> Option<&str> {
            self.repo().url()
        }

        pub fn id(&self) -> Option<&str> {
            self.repo().id()
        }

        pub fn name(&self) -> Option<&str> {
            self.repo().name()
        }

        pub fn get_versions_of(&self, package: &str) -> impl Iterator<Item = &'_ PackageJson> {
            self.repo().get_versions_of(package)
        }

        pub fn get_packages(&self) -> impl Iterator<Item = &'_ PackageVersions> {
            self.repo().get_packages()
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Default)]
    pub struct VrcGetMeta {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        pub etag: String,
    }
}

pub mod repository {
    use super::*;
    use serde::{Deserializer, Serializer};

    #[derive(Debug, Clone)]
    pub struct Repository {
        actual: JsonMap,
        parsed: ParsedRepository,
    }

    impl Repository {
        pub fn new(cache: JsonMap) -> serde_json::Result<Self> {
            Ok(Self {
                parsed: serde_json::from_value(Value::Object(cache.clone()))?,
                actual: cache,
            })
        }

        pub fn set_id_if_none(&mut self, f: impl FnOnce() -> String) {
            if let None = self.parsed.id {
                let id = f();
                self.parsed.id = Some(id.clone());
                self.actual.insert("id".to_owned(), Value::String(id));
            }
        }

        pub fn set_url_if_none(&mut self, f: impl FnOnce() -> String) {
            if let None = self.parsed.url {
                let url = f();
                self.parsed.url = Some(url.clone());
                self.actual.insert("url".to_owned(), Value::String(url));
                if let None = self.parsed.id {
                    let url = self.parsed.url.clone().unwrap();
                    self.set_id_if_none(move || url);
                }
            }
        }

        pub fn url(&self) -> Option<&str> {
            self.parsed.url.as_deref()
        }

        pub fn id(&self) -> Option<&str> {
            self.parsed.id.as_deref()
        }

        pub fn name(&self) -> Option<&str> {
            self.parsed.name.as_deref()
        }

        pub fn get_versions_of(
            &self,
            package: &str,
        ) -> impl Iterator<Item = &'_ package::PackageJson> {
            self.parsed
                .packages
                .get(package)
                .map(|x| x.versions.values())
                .into_iter()
                .flatten()
        }

        pub fn get_packages(&self) -> impl Iterator<Item = &'_ PackageVersions> {
            self.parsed.packages.values().into_iter()
        }
    }

    impl Serialize for Repository {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            self.actual.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Repository {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            use serde::de::Error;
            let map = JsonMap::deserialize(deserializer)?;
            Self::new(map).map_err(Error::custom)
        }
    }

    #[derive(Deserialize, Debug, Clone)]
    struct ParsedRepository {
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        url: Option<String>,
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        packages: HashMap<String, PackageVersions>,
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct PackageVersions {
        #[serde(default)]
        pub versions: HashMap<String, package::PackageJson>,
    }
}

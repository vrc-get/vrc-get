use crate::version::{Version, VersionRange};
use indexmap::IndexMap;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::PathBuf;

type JsonMap = Map<String, Value>;

pub mod manifest {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct VpmDependency {
        pub version: Version,
    }

    impl VpmDependency {
        pub fn new(version: Version) -> Self {
            Self { version }
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

        #[serde(rename = "legacyFolders")]
        #[serde(default)]
        pub legacy_folders: HashMap<String, String>,
        #[serde(rename = "legacyFiles")]
        #[serde(default)]
        pub legacy_files: HashMap<String, String>,
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
    }

    impl UserRepoSetting {
        pub fn new(local_path: PathBuf, name: Option<String>, url: Option<String>, id: Option<String>) -> Self {
            Self {
                local_path,
                name,
                id: id.or(url.clone()),
                url,
            }
        }
    }
}

pub mod repository {
    use serde::{Deserializer, Serializer};
    use crate::vpm::structs::package::PackageJson;
    use crate::vpm::structs::remote_repo::{PackageVersions, ParsedRepository};
    use super::*;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct LocalCachedRepository {
        repo: Repository,
        headers: IndexMap<String, String>,
        #[serde(rename = "vrc-get")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub vrc_get: Option<VrcGetMeta>,
    }

    impl LocalCachedRepository {
        pub fn new(repo: JsonMap, id: Option<String>, url: Option<String>) -> serde_json::Result<Self> {
            Ok(Self {
                repo: Repository::new(repo, id, url)?,
                headers: IndexMap::new(),
                vrc_get: None,
            })
        }

        pub fn url(&self) -> Option<&str> {
            self.repo.parsed.url.as_deref()
        }

        pub fn id(&self) -> Option<&str> {
            self.repo.parsed.id.as_deref()
        }

        pub fn name(&self) -> Option<&str> {
            self.repo.parsed.name.as_deref()
        }

        pub fn set_repo(&mut self, repo: JsonMap) -> serde_json::Result<()> {
            self.repo = Repository::new(
                repo,
                self.id().map(|x| x.to_owned()),
                self.url().map(|x| x.to_owned()),
            )?;
            Ok(())
        }

        pub fn get_versions_of(&self, package: &str) -> impl Iterator<Item = &'_ PackageJson> {
            self.repo.parsed.packages
                .get(package)
                .map(|x| x.versions.values())
                .into_iter()
                .flatten()
        }

        pub fn get_packages(&self) -> impl Iterator<Item = &'_ PackageVersions> {
            self.repo.parsed.packages
                .values()
                .into_iter()
        }
    }

    #[derive(Debug, Clone)]
    struct Repository {
        actual: JsonMap,
        parsed: ParsedRepository,
    }

    impl Repository {
        pub fn new(mut cache: JsonMap, mut id: Option<String>, url: Option<String>) -> serde_json::Result<Self> {
            // initialize url and id if not specified

            id = id.or_else(|| url.as_ref().map(|x| x.clone()));

            if let (None, Some(url)) = (cache.get("url"), url) {
                cache.insert("url".to_owned(), Value::String(url));
            }
            
            if let (None, Some(id)) = (cache.get("id"), id) {
                cache.insert("id".to_owned(), Value::String(id));
            }

            Ok(Self {
                parsed: serde_json::from_value(Value::Object(cache.clone()))?,
                actual: cache,
            })
        }
    }

    impl Serialize for Repository {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
            self.actual.serialize(serializer)
        }
    }

    impl <'de> Deserialize<'de> for Repository {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
            use serde::de::Error;
            let map = JsonMap::deserialize(deserializer)?;
            Self::new(map, None, None).map_err(Error::custom)
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Default)]
    pub struct VrcGetMeta {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        pub etag: String,
    }
}

pub mod remote_repo {
    use super::*;

    #[derive(Deserialize, Debug, Clone)]
    pub struct ParsedRepository {
        pub name: Option<String>,
        pub url: Option<String>,
        pub id: Option<String>,
        pub author: Option<String>,
        pub packages: HashMap<String, PackageVersions>,
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct PackageVersions {
        #[serde(default)]
        pub versions: HashMap<String, package::PackageJson>,
    }
}

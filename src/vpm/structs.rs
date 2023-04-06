use crate::version::{Version, VersionRange};
use indexmap::IndexMap;
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
        pub vpm_dependencies: Option<IndexMap<String, VersionRange>>,
        #[serde(default)]
        pub url: String,
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
    }

    impl UserRepoSetting {
        pub fn new(local_path: PathBuf, name: Option<String>, url: Option<String>) -> Self {
            Self {
                local_path,
                name,
                url,
            }
        }
    }
}

pub mod repository {
    use serde::{Deserializer, Serializer};
    use crate::vpm::structs::remote_repo::PackageVersions;
    use super::*;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct LocalCachedRepository {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub repo: Option<JsonMap>,
        #[serde(default, skip_serializing_if = "RepositoryCache::is_empty")]
        pub cache: RepositoryCache,
        #[serde(rename = "CreationInfo")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub creation_info: Option<CreationInfo>,
        #[serde(rename = "Description")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<Description>,
        #[serde(rename = "vrc-get")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub vrc_get: Option<VrcGetMeta>,
    }

    impl LocalCachedRepository {
        pub fn new(path: PathBuf, name: Option<String>, url: Option<String>) -> Self {
            Self {
                repo: None,
                cache: RepositoryCache::default(),
                creation_info: Some(CreationInfo {
                    local_path: Some(path),
                    url,
                    name: name.clone(),
                }),
                description: Some(Description {
                    name,
                    r#type: Some("JsonRepo".to_owned()),
                }),
                vrc_get: None,
            }
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct RepositoryCache {
        actual: JsonMap,
        parsed: IndexMap<String, remote_repo::PackageVersions>,
    }

    impl RepositoryCache {
        pub fn new(cache: JsonMap) -> serde_json::Result<Self> {
            Ok(Self {
                parsed: serde_json::from_value(Value::Object(cache.clone()))?,
                actual: cache,
            })
        }

        pub fn is_empty(&self) -> bool {
            self.actual.is_empty()
        }

        pub fn parsed(&self) -> &IndexMap<String, PackageVersions> {
            &self.parsed
        }

        pub fn get(&self, key: &str) -> Option<&PackageVersions> {
            self.parsed.get(key)
        }

        pub fn values(&self) -> impl Iterator<Item = &PackageVersions> {
            self.parsed.values()
        }
    }

    impl Serialize for RepositoryCache {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
            self.actual.serialize(serializer)
        }
    }

    impl <'de> Deserialize<'de> for RepositoryCache {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
            use serde::de::Error;
            let map = JsonMap::deserialize(deserializer)?;
            Self::new(map).map_err(Error::custom)
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct CreationInfo {
        #[serde(rename = "localPath")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub local_path: Option<PathBuf>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Description {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub r#type: Option<String>,
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
    pub struct PackageVersions {
        #[serde(default)]
        pub versions: IndexMap<String, package::PackageJson>,
    }
}

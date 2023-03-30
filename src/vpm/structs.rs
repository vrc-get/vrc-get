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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub display_name: Option<String>,
        pub description: Option<String>,
        pub version: Version,
        #[serde(rename = "vpmDependencies")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub vpm_dependencies: Option<IndexMap<String, VersionRange>>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
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
        // TODO: check property name when VCC 2.1.0 is released
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub id: Option<String>,
    }

    impl UserRepoSetting {
        pub fn new(local_path: PathBuf, name: Option<String>, url: Option<String>, id: Option<String>) -> Self {
            Self {
                local_path,
                name,
                url,
                id,
            }
        }
    }
}

pub mod repository {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct LocalCachedRepository {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub repo: Option<JsonMap>,
        #[serde(default, skip_serializing_if = "JsonMap::is_empty")]
        pub cache: JsonMap,
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
                cache: JsonMap::new(),
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

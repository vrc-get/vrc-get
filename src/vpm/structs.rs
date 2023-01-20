pub use crate::vpm::version::VersionRange;
use indexmap::IndexMap;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

pub type Rest = IndexMap<String, Value>;

pub mod manifest {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, Clone, Default)]
    pub struct VpmManifest {
        #[serde(default)]
        pub dependencies: IndexMap<String, VpmDependency>,
        #[serde(default)]
        pub locked: IndexMap<String, VpmLockedDependency>,
        #[serde(flatten)]
        pub(crate) rest: Rest,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct VpmDependency {
        pub version: Version,
        #[serde(flatten)]
        pub(crate) rest: Rest,
    }

    impl VpmDependency {
        pub fn dummy() -> Self {
            Self {
                version: Version::new(0, 0, 0),
                rest: Rest::new(),
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct VpmLockedDependency {
        pub version: Version,
        #[serde(default, skip_serializing_if = "indexmap::IndexMap::is_empty")]
        pub dependencies: IndexMap<String, VersionRange>,
        #[serde(flatten)]
        pub(crate) rest: Rest,
    }

    impl VpmLockedDependency {
        pub fn dummy() -> Self {
            Self {
                version: Version::new(0, 0, 0),
                dependencies: IndexMap::new(),
                rest: Rest::new(),
            }
        }
    }
}

pub mod package {
    use super::*;
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct PackageJson {
        pub name: String,
        #[serde(rename = "displayName")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub display_name: Option<String>,
        pub version: Version,
        #[serde(rename = "vpmDependencies")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub vpm_dependencies: Option<IndexMap<String, VersionRange>>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        pub url: String,
        #[serde(flatten)]
        pub(crate) rest: Rest,
    }
}

pub mod setting {
    use super::*;
    #[derive(Serialize, Deserialize, Debug, Clone, Default)]
    pub struct SettingsJson {
        #[serde(rename = "userRepos")]
        #[serde(default)]
        pub user_repos: Vec<UserRepoSetting>,
        #[serde(flatten)]
        pub(crate) rest: Rest,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct UserRepoSetting {
        #[serde(rename = "localPath")]
        pub local_path: PathBuf,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        // must be non-relative url.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub url: Option<String>,
        #[serde(flatten)]
        pub(crate) rest: Rest,
    }
}

pub mod repository {
    use super::*;
    use std::rc::Rc;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct LocalCachedRepository {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub repo: Option<RemoteRepository>,
        #[serde(default, skip_serializing_if = "indexmap::IndexMap::is_empty")]
        pub cache: IndexMap<String, PackageVersions>,
        #[serde(rename = "CreationInfo")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub creation_info: Option<CreationInfo>,
        #[serde(rename = "Description")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<Description>,
        #[serde(flatten)]
        pub(crate) rest: Rest,
    }

    impl LocalCachedRepository {
        pub fn new(path: PathBuf, name: Option<String>, url: Option<String>) -> Self {
            Self {
                repo: None,
                cache: IndexMap::new(),
                creation_info: Some(CreationInfo {
                    local_path: Some(path),
                    url,
                    name: name.clone(),
                    rest: Rest::new(),
                }),
                description: Some(Description {
                    name,
                    r#type: Some("JsonRepo".to_owned()),
                    rest: Rest::new(),
                }),
                rest: Rest::new(),
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct RemoteRepository {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub author: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub url: Option<String>,
        #[serde(default)]
        pub packages: IndexMap<String, PackageVersions>,
        #[serde(flatten)]
        pub(crate) rest: Rest,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct PackageVersions {
        #[serde(default)]
        pub versions: IndexMap<String, Rc<package::PackageJson>>,
        #[serde(flatten)]
        pub(crate) rest: Rest,
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
        #[serde(flatten)]
        pub(crate) rest: Rest,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Description {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub r#type: Option<String>,
        #[serde(flatten)]
        pub(crate) rest: Rest,
    }
}


use serde::{Deserialize, Serialize};
use serde_json::Value;

type Rest = indexmap::IndexMap<String, Value>;

pub mod manifest {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, Default)]
    pub struct VpmManifest {
        #[serde(default)]
        dependencies: indexmap::IndexMap<String, VpmDependency>,
        #[serde(default)]
        locked: indexmap::IndexMap<String, VpmLockedDependency>,
        #[serde(flatten)]
        rest: Rest,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct VpmDependency {
        version: String,
        #[serde(flatten)]
        rest: Rest,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct VpmLockedDependency {
        version: String,
        #[serde(default, skip_serializing_if = "indexmap::IndexMap::is_empty")]
        dependencies: indexmap::IndexMap<String, String>,
        #[serde(flatten)]
        rest: Rest,
    }
}

pub mod package {
    use super::*;
    #[derive(Serialize, Deserialize, Debug)]
    pub struct PackageJson {
        name: String,
        #[serde(rename = "displayName")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<String>,
        #[serde(rename = "vpmDependencies")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        vpm_dependencies: Option<indexmap::IndexMap<String, String>>,
        #[serde(flatten)]
        rest: Rest,
    }
}

pub mod setting {
    use super::*;
    #[derive(Serialize, Deserialize, Debug)]
    pub struct SettingsJson {
        #[serde(rename = "userRepos")]
        #[serde(default)]
        user_repos: Vec<UserRepoSetting>,
        #[serde(flatten)]
        rest: Rest,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct UserRepoSetting {
        #[serde(rename = "localPath")]
        local_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        // must be non-relative url.
        #[serde(default)]
        url: String,
        #[serde(flatten)]
        rest: Rest,
    }
}

pub mod repository {
    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct LocalCachedRepository {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        repo: Option<RemoteRepository>,
        #[serde(default, skip_serializing_if = "indexmap::IndexMap::is_empty")]
        cache: indexmap::IndexMap<String, PackageVersions>,
        #[serde(rename="CreationInfo")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        creation_info: Option<CreationInfo>,
        #[serde(rename="Description")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<Description>,
        #[serde(flatten)]
        rest: Rest,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct RemoteRepository {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        author: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(default)]
        packages: indexmap::IndexMap<String, PackageVersions>,
        #[serde(flatten)]
        rest: Rest,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct PackageVersions {
        #[serde(default)]
        versions: indexmap::IndexMap<String, package::PackageJson>,
        #[serde(flatten)]
        rest: Rest,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct CreationInfo {
        #[serde(rename="localPath")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        local_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(flatten)]
        rest: Rest,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Description {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        r#type: Option<String>,
        #[serde(flatten)]
        rest: Rest,
    }
}

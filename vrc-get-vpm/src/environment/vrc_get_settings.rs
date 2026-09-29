use crate::io;
use crate::io::DefaultEnvironmentIo;
use crate::utils::load_json_or_default;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

type JsonObject = Map<String, Value>;

/// since this file is vrc-get specific, additional keys can be removed
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AsJson {
    #[serde(default)]
    ignore_official_repository: bool,
    #[serde(default)]
    ignore_curated_repository: bool,
    #[cfg(feature = "experimental-project-management")]
    #[serde(default)]
    project_list_sync_mode: super::project_management::SyncWithLitedbMode,

    #[serde(flatten)]
    rest: JsonObject,
}

#[derive(Debug, Clone)]
pub(crate) struct VrcGetSettings {
    parsed: AsJson,
}

const JSON_PATH: &str = "vrc-get/settings.json";

impl VrcGetSettings {
    pub async fn load(io: &DefaultEnvironmentIo) -> io::Result<Self> {
        let parsed = load_json_or_default(io, JSON_PATH.as_ref()).await?;

        Ok(Self { parsed })
    }

    pub fn ignore_official_repository(&self) -> bool {
        self.parsed.ignore_official_repository
    }

    pub fn ignore_curated_repository(&self) -> bool {
        self.parsed.ignore_curated_repository
    }

    #[cfg(feature = "experimental-project-management")]
    pub fn project_list_sync_mode(&self) -> super::project_management::SyncWithLitedbMode {
        self.parsed.project_list_sync_mode
    }
}

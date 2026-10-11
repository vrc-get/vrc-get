use crate::io;
use crate::io::DefaultEnvironmentIo;
use crate::utils::{load_json_or_default, save_json};
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

    #[serde(flatten)]
    rest: JsonObject,
}

#[derive(Debug, Clone)]
pub(crate) struct VrcGetSettings {
    parsed: AsJson,
    should_save: bool,
}

const JSON_PATH: &str = "vrc-get/settings.json";

impl VrcGetSettings {
    pub async fn load(io: &DefaultEnvironmentIo) -> io::Result<Self> {
        let parsed = load_json_or_default(io, JSON_PATH.as_ref()).await?;

        Ok(Self {
            parsed,
            should_save: false,
        })
    }

    pub fn ignore_official_repository(&self) -> bool {
        self.parsed.ignore_official_repository
    }

    pub fn ignore_curated_repository(&self) -> bool {
        self.parsed.ignore_curated_repository
    }

    pub async fn save(&self, io: &DefaultEnvironmentIo) -> io::Result<()> {
        if self.should_save {
            save_json(io, JSON_PATH.as_ref(), &self.parsed).await?;
        }
        Ok(())
    }
}

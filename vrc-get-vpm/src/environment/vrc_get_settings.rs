use crate::io;
use crate::io::{DefaultEnvironmentIo, IoTrait};
use crate::utils::{parse_json_file, read_to_end};
use serde::{Deserialize, Serialize};

/// since this file is vrc-get specific, additional keys can be removed
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AsJson {
    #[serde(default)]
    ignore_official_repository: bool,
    #[serde(default)]
    ignore_curated_repository: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct VrcGetSettings {
    parsed: AsJson,
}

const JSON_PATH: &str = "vrc-get/settings.json";

impl VrcGetSettings {
    pub async fn load(io: &DefaultEnvironmentIo) -> io::Result<Self> {
        //let parsed = load_json_or_default(io, JSON_PATH.as_ref()).await?;

        let parsed = match io.open(JSON_PATH.as_ref()).await {
            Ok(file) => match read_to_end(file).await? {
                vec if vec.is_empty() => Default::default(),
                vec => {
                    log::warn!("vrc-get specific settings file is experimental feature!");
                    parse_json_file(&vec, JSON_PATH.as_ref())?
                }
            },
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => Default::default(),
            Err(e) => return Err(e),
        };

        Ok(Self { parsed })
    }

    pub fn ignore_official_repository(&self) -> bool {
        self.parsed.ignore_official_repository
    }

    pub fn ignore_curated_repository(&self) -> bool {
        self.parsed.ignore_curated_repository
    }
}

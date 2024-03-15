use crate::io;
use crate::io::EnvironmentIo;
use crate::utils::{read_json_file, SaveController};
use serde::{Deserialize, Serialize};

/// since this file is vrc-get specific, additional keys can be removed
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AsJson {
    #[serde(default)]
    ignore_official_repository: bool,
    #[serde(default)]
    ignore_curated_repository: bool,
}

#[derive(Debug)]
pub(crate) struct VrcGetSettings {
    controller: SaveController<AsJson>,
}

const JSON_PATH: &str = "vrc-get/settings.json";

impl VrcGetSettings {
    pub async fn load(io: &impl EnvironmentIo) -> io::Result<Self> {
        //let parsed = load_json_or_default(io, JSON_PATH.as_ref()).await?;

        let parsed = match io.open(JSON_PATH.as_ref()).await {
            Ok(file) => {
                log::warn!("vrc-get specific settings file is experimental feature!");
                read_json_file::<AsJson>(file, JSON_PATH.as_ref()).await?
            }
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => Default::default(),
            Err(e) => return Err(e),
        };

        Ok(Self {
            controller: SaveController::new(parsed),
        })
    }

    pub fn ignore_official_repository(&self) -> bool {
        self.controller.ignore_official_repository
    }

    #[allow(dead_code)]
    pub fn set_ignore_official_repository(&mut self, value: bool) {
        self.controller.as_mut().ignore_official_repository = value;
    }

    pub fn ignore_curated_repository(&self) -> bool {
        self.controller.ignore_curated_repository
    }

    #[allow(dead_code)]
    pub fn set_ignore_curated_repository(&mut self, value: bool) {
        self.controller.as_mut().ignore_curated_repository = value;
    }

    pub async fn save(&mut self, io: &impl EnvironmentIo) -> io::Result<()> {
        self.controller.save(io, JSON_PATH.as_ref()).await
    }
}

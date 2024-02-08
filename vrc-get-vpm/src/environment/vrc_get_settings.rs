use crate::io;
use crate::io::EnvironmentIo;
use crate::utils::{read_json_file, to_vec_pretty_os_eol};
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
    as_json: AsJson,

    settings_changed: bool,
}

const JSON_PATH: &str = "vrc-get-settings.json";

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
            as_json: parsed,
            settings_changed: false,
        })
    }

    pub fn ignore_official_repository(&self) -> bool {
        self.as_json.ignore_official_repository
    }

    #[allow(dead_code)]
    pub fn set_ignore_official_repository(&mut self, value: bool) {
        self.as_json.ignore_official_repository = value;
        self.settings_changed = true;
    }

    pub fn ignore_curated_repository(&self) -> bool {
        self.as_json.ignore_curated_repository
    }

    #[allow(dead_code)]
    pub fn set_ignore_curated_repository(&mut self, value: bool) {
        self.as_json.ignore_curated_repository = value;
        self.settings_changed = true;
    }

    pub async fn save(&mut self, io: &impl EnvironmentIo) -> io::Result<()> {
        if !self.settings_changed {
            return Ok(());
        }

        io.create_dir_all(".".as_ref()).await?;
        io.write(JSON_PATH.as_ref(), &to_vec_pretty_os_eol(&self.as_json)?)
            .await?;

        self.settings_changed = false;
        Ok(())
    }
}

use crate::io::EnvironmentIo;
use crate::utils::{load_json_or_default, to_vec_pretty_os_eol};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;

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

    path: PathBuf,

    settings_changed: bool,
}

impl VrcGetSettings {
    pub async fn load(json_path: PathBuf) -> io::Result<Self> {
        let parsed = load_json_or_default(&json_path).await?;

        Ok(Self {
            as_json: parsed,
            path: json_path,
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

        let json_path = &self.path;

        if let Some(parent) = json_path.parent() {
            io.create_dir_all(&parent).await?;
        }

        io.write(json_path, &to_vec_pretty_os_eol(&self.as_json)?)
            .await?;
        self.settings_changed = false;
        Ok(())
    }
}

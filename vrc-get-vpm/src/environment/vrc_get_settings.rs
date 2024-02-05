use crate::utils::{load_json_or_default, to_vec_pretty_os_eol};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;
use tokio::fs::create_dir_all;

/// since this file is vrc-get specific, additional keys can be removed
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AsJson {}

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

    pub async fn save(&mut self) -> io::Result<()> {
        if !self.settings_changed {
            return Ok(());
        }

        let json_path = &self.path;

        if let Some(parent) = json_path.parent() {
            create_dir_all(&parent).await?;
        }

        tokio::fs::write(json_path, &to_vec_pretty_os_eol(&self.as_json)?).await?;
        self.settings_changed = false;
        Ok(())
    }
}

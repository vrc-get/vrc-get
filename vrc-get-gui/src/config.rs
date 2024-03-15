use futures::AsyncReadExt;
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use vrc_get_vpm::io::{DefaultEnvironmentIo, EnvironmentIo, IoTrait};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiConfig {
    #[serde(default)]
    pub gui_hidden_repositories: IndexSet<String>,
    #[serde(default)]
    pub hide_local_user_packages: bool,
}

pub struct GuiConfigHandler<'a> {
    config: &'a mut GuiConfig,
    path: &'a PathBuf,
}

impl GuiConfigHandler<'_> {
    pub async fn save(&self) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.config)?;
        tokio::fs::write(&self.path, json.as_bytes()).await
    }
}

impl Deref for GuiConfigHandler<'_> {
    type Target = GuiConfig;

    #[inline(always)]
    fn deref(&self) -> &GuiConfig {
        self.config
    }
}

impl DerefMut for GuiConfigHandler<'_> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut GuiConfig {
        self.config
    }
}

pub struct GuiConfigHolder {
    cached_value: Option<(GuiConfig, PathBuf)>,
}

impl GuiConfigHolder {
    pub fn new() -> Self {
        Self { cached_value: None }
    }

    pub async fn load(&mut self, io: &DefaultEnvironmentIo) -> io::Result<GuiConfigHandler> {
        let (config, path) = if let Some((ref mut config, ref path)) = self.cached_value {
            (config, path)
        } else {
            let path = io.resolve("vrc-get-gui-config.json".as_ref());
            let value = match io.open(&path).await {
                Ok(mut file) => {
                    let mut buffer = Vec::new();
                    file.read_to_end(&mut buffer).await?;
                    serde_json::from_slice(&buffer)?
                }
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => GuiConfig::default(),
                Err(e) => return Err(e),
            };
            self.cached_value = Some((value, path));
            let (config, path) = self.cached_value.as_mut().unwrap();
            (config, &*path)
        };

        Ok(GuiConfigHandler { config, path })
    }
}

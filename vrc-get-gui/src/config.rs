use futures::AsyncReadExt;
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use vrc_get_vpm::io::{DefaultEnvironmentIo, EnvironmentIo, IoTrait};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiConfig {
    #[serde(default)]
    pub gui_hidden_repositories: IndexSet<String>,
    #[serde(default)]
    pub hide_local_user_packages: bool,
    #[serde(default)]
    pub window_size: WindowSize,
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default = "language_default")]
    pub language: String,
    #[serde(default = "backup_default")]
    pub backup_format: String,
    #[serde(default = "project_sorting_default")]
    pub project_sorting: String,
}

impl Default for GuiConfig {
    fn default() -> Self {
        GuiConfig {
            gui_hidden_repositories: IndexSet::new(),
            hide_local_user_packages: false,
            window_size: WindowSize::default(),
            fullscreen: false,
            language: language_default(),
            backup_format: backup_default(),
            project_sorting: project_sorting_default(),
        }
    }
}

impl GuiConfig {
    fn fix_defaults(&mut self) {
        if self.language.is_empty() {
            self.language = language_default();
        }
        if self.language == "zh_cn" {
            self.language = "zh_hans".to_string();
        }
        if self.backup_format.is_empty() {
            self.backup_format = backup_default();
        }
        if self.project_sorting.is_empty() {
            self.project_sorting = project_sorting_default();
        }
    }
}

fn language_default() -> String {
    for locale in sys_locale::get_locales() {
        if locale.starts_with("en") {
            return "en".to_string();
        }
        if locale.starts_with("de") {
            return "de".to_string();
        }
        if locale.starts_with("ja") {
            return "ja".to_string();
        }
        if locale.starts_with("zh") {
            return "zh_hans".to_string();
        }
    }

    "en".to_string()
}

fn backup_default() -> String {
    "default".to_string()
}

fn project_sorting_default() -> String {
    "lastModified".to_string()
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl Default for WindowSize {
    fn default() -> Self {
        WindowSize {
            width: 1000,
            height: 800,
        }
    }
}

pub struct GuiConfigHandler<'a> {
    config: &'a mut GuiConfig,
    path: &'a PathBuf,
}

impl GuiConfigHandler<'_> {
    pub async fn save(&self) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.config)?;
        tokio::fs::create_dir_all(self.path.parent().unwrap()).await?;
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
            let path = io.resolve("vrc-get/gui-config.json".as_ref());
            let value = match io.open(&path).await {
                Ok(mut file) => {
                    let mut buffer = Vec::new();
                    file.read_to_end(&mut buffer).await?;
                    let mut loaded = serde_json::from_slice::<GuiConfig>(&buffer)?;
                    loaded.fix_defaults();
                    loaded
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

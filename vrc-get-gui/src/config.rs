use crate::logging::LogLevel;
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    #[serde(default = "theme_default")]
    pub theme: String,
    #[serde(default = "backup_default")]
    pub backup_format: String,
    #[serde(default = "project_sorting_default")]
    pub project_sorting: String,
    #[serde(default = "release_channel_default")]
    // "stable" or "beta"
    pub release_channel: String,
    #[serde(default)]
    pub use_alcom_for_vcc_protocol: bool,
    #[serde(default)]
    pub setup_process_progress: u32,
    #[serde(default)]
    pub default_unity_arguments: Option<Vec<String>>,
    #[serde(default = "log_level_default")]
    pub logs_level: Vec<LogLevel>,
    #[serde(default = "gui_animation_default")]
    pub gui_animation: bool,
    #[serde(default)]
    pub unity_hub_access_method: UnityHubAccessMethod,
    // last element is the most recent one
    // 8 paths are saved
    #[serde(default)]
    pub recent_project_locations: Vec<String>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Default, specta::Type)]
pub enum UnityHubAccessMethod {
    /// Reads config files of Unity Hub
    #[default]
    ReadConfig,
    /// Launches headless Unity Hub in background
    CallHub,
}

impl Default for GuiConfig {
    fn default() -> Self {
        GuiConfig {
            gui_hidden_repositories: IndexSet::new(),
            hide_local_user_packages: false,
            window_size: WindowSize::default(),
            fullscreen: false,
            language: language_default(),
            theme: theme_default(),
            backup_format: backup_default(),
            project_sorting: project_sorting_default(),
            release_channel: release_channel_default(),
            use_alcom_for_vcc_protocol: false,
            setup_process_progress: 0,
            default_unity_arguments: None,
            logs_level: log_level_default(),
            gui_animation: true,
            unity_hub_access_method: UnityHubAccessMethod::ReadConfig,
            recent_project_locations: Vec::new(),
        }
    }
}

impl GuiConfig {
    pub(crate) fn fix_defaults(&mut self) {
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

fn theme_default() -> String {
    "system".to_string()
}

fn backup_default() -> String {
    "default".to_string()
}

fn project_sorting_default() -> String {
    "lastModified".to_string()
}

fn release_channel_default() -> String {
    "stable".to_string()
}

fn log_level_default() -> Vec<LogLevel> {
    vec![
        LogLevel::Debug,
        LogLevel::Error,
        LogLevel::Warn,
        LogLevel::Info,
    ]
}

fn gui_animation_default() -> bool {
    true
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl Default for WindowSize {
    fn default() -> Self {
        WindowSize {
            width: 1300,
            height: 800,
        }
    }
}

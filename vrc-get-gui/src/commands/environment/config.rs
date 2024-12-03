use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::prelude::*;
use crate::logging::LogLevel;

#[tauri::command]
#[specta::specta]
pub async fn environment_language(config: State<'_, GuiConfigState>) -> Result<String, RustError> {
    Ok(config.get().language.clone())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_language(
    config: State<'_, GuiConfigState>,
    language: String,
) -> Result<(), RustError> {
    let mut config = config.load_mut().await?;
    config.language = language;
    config.save().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_theme(config: State<'_, GuiConfigState>) -> Result<String, RustError> {
    Ok(config.get().theme.clone())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_theme(
    config: State<'_, GuiConfigState>,
    theme: String,
) -> Result<(), RustError> {
    let mut config = config.load_mut().await?;
    config.theme = theme;
    config.save().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_get_project_sorting(
    config: State<'_, GuiConfigState>,
) -> Result<String, RustError> {
    Ok(config.get().project_sorting.clone())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_project_sorting(
    config: State<'_, GuiConfigState>,
    sorting: String,
) -> Result<(), RustError> {
    let mut config = config.load_mut().await?;
    config.project_sorting = sorting;
    config.save().await?;
    Ok(())
}

#[derive(Serialize, Deserialize, specta::Type, Copy, Clone)]
pub enum SetupPages {
    Appearance,
    UnityHub,
    ProjectPath,
    Backups,
    SystemSetting,
}

impl SetupPages {
    pub fn as_flag(&self) -> u32 {
        match self {
            SetupPages::Appearance => 0x00000001,
            SetupPages::UnityHub => 0x00000002,
            SetupPages::ProjectPath => 0x00000004,
            SetupPages::Backups => 0x00000008,
            SetupPages::SystemSetting => 0x00000010,
        }
    }

    pub fn is_finished(&self, flags: u32) -> bool {
        flags & self.as_flag() == self.as_flag()
    }

    pub fn pages() -> &'static [SetupPages] {
        if cfg!(target_os = "macos") {
            &[
                SetupPages::Appearance,
                SetupPages::UnityHub,
                SetupPages::ProjectPath,
                SetupPages::Backups,
            ]
        } else {
            &[
                SetupPages::Appearance,
                SetupPages::UnityHub,
                SetupPages::ProjectPath,
                SetupPages::Backups,
                SetupPages::SystemSetting,
            ]
        }
    }

    pub fn path(self) -> &'static str {
        match self {
            SetupPages::Appearance => "/setup/appearance/",
            SetupPages::UnityHub => "/setup/unity-hub/",
            SetupPages::ProjectPath => "/setup/project-path/",
            SetupPages::Backups => "/setup/backups/",
            SetupPages::SystemSetting => "/setup/system-setting/",
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn environment_get_finished_setup_pages(
    config: State<'_, GuiConfigState>,
) -> Result<Vec<SetupPages>, RustError> {
    let setup_process_progress = config.get().setup_process_progress;

    Ok(SetupPages::pages()
        .iter()
        .copied()
        .filter(|page| page.is_finished(setup_process_progress))
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_finished_setup_page(
    config: State<'_, GuiConfigState>,
    page: SetupPages,
) -> Result<(), RustError> {
    let mut config = config.load_mut().await?;
    config.setup_process_progress |= page.as_flag();
    config.save().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_clear_setup_process(
    config: State<'_, GuiConfigState>,
) -> Result<(), RustError> {
    let mut config = config.load_mut().await?;
    config.setup_process_progress = 0;
    config.save().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_logs_level(
    config: State<'_, GuiConfigState>,
) -> Result<Vec<LogLevel>, RustError> {
    Ok(config.get().logs_level.clone())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_logs_level(
    config: State<'_, GuiConfigState>,
    logs_level: Vec<LogLevel>,
) -> Result<(), RustError> {
    let mut config = config.load_mut().await?;
    config.logs_level = logs_level;
    config.save().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_logs_auto_scroll(
    config: State<'_, GuiConfigState>,
) -> Result<bool, RustError> {
    Ok(config.get().logs_auto_scroll.clone())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_logs_auto_scroll(
    config: State<'_, GuiConfigState>,
    logs_auto_scroll: bool,
) -> Result<(), RustError> {
    let mut config = config.load_mut().await?;
    config.logs_auto_scroll = logs_auto_scroll;
    config.save().await?;
    Ok(())
}

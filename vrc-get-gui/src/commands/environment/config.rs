use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex;

use crate::commands::prelude::*;

#[tauri::command]
#[specta::specta]
pub async fn environment_language(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<String, RustError> {
    with_config!(state, |config| Ok(config.language.clone()))
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_language(
    state: State<'_, Mutex<EnvironmentState>>,
    language: String,
) -> Result<(), RustError> {
    with_config!(state, |mut config| {
        config.language = language;
        config.save().await?;
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub async fn environment_theme(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<String, RustError> {
    with_config!(state, |config| Ok(config.theme.clone()))
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_theme(
    state: State<'_, Mutex<EnvironmentState>>,
    theme: String,
) -> Result<(), RustError> {
    with_config!(state, |mut config| {
        config.theme = theme;
        config.save().await?;
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub async fn environment_get_project_sorting(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<String, RustError> {
    with_config!(state, |config| Ok(config.project_sorting.clone()))
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_project_sorting(
    state: State<'_, Mutex<EnvironmentState>>,
    sorting: String,
) -> Result<(), RustError> {
    with_config!(state, |mut config| {
        config.project_sorting = sorting;
        config.save().await?;
        Ok(())
    })
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
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<Vec<SetupPages>, RustError> {
    let setup_process_progress = with_config!(state, |config| config.setup_process_progress);

    Ok(SetupPages::pages()
        .iter()
        .copied()
        .filter(|page| page.is_finished(setup_process_progress))
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_finished_setup_page(
    state: State<'_, Mutex<EnvironmentState>>,
    page: SetupPages,
) -> Result<(), RustError> {
    with_config!(state, |mut config| {
        config.setup_process_progress |= page.as_flag();
        config.save().await?;
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub async fn environment_clear_setup_process(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<(), RustError> {
    with_config!(state, |mut config| {
        config.setup_process_progress = 0;
        config.save().await?;
        Ok(())
    })
}

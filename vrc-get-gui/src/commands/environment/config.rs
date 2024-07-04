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

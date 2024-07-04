use std::ptr::NonNull;

use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;
use url::Url;

use vrc_get_vpm::repository::RemoteRepository;
use vrc_get_vpm::{PackageCollection, PackageInfo, VersionSelector};

use crate::commands::prelude::*;
use crate::specta::IndexMapV2;

#[derive(Serialize, specta::Type)]
pub struct TauriPackage {
    env_version: u32,
    index: usize,

    #[serde(flatten)]
    base: TauriBasePackageInfo,

    source: TauriPackageSource,
}

#[derive(Serialize, specta::Type)]
enum TauriPackageSource {
    LocalUser,
    Remote { id: String, display_name: String },
}

impl TauriPackage {
    fn new(env_version: u32, index: usize, package: &PackageInfo) -> Self {
        let source = if let Some(repo) = package.repo() {
            let id = repo.id().or(repo.url().map(|x| x.as_str())).unwrap();
            TauriPackageSource::Remote {
                id: id.to_string(),
                display_name: repo.name().unwrap_or(id).to_string(),
            }
        } else {
            TauriPackageSource::LocalUser
        };

        Self {
            env_version,
            index,
            base: TauriBasePackageInfo::new(package.package_json()),
            source,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn environment_refetch_packages(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<(), RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    env_state
        .environment
        .get_environment_mut(UpdateRepositoryMode::Force, &env_state.io)
        .await?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_packages(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<Vec<TauriPackage>, RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    let environment = env_state
        .environment
        .get_environment_mut(UpdateRepositoryMode::IfOutdatedOrNecessary, &env_state.io)
        .await?;

    let packages = environment
        .get_all_packages()
        .collect::<Vec<_>>()
        .into_boxed_slice();
    if let Some(ptr) = env_state.packages {
        env_state.packages = None; // avoid a double drop
        unsafe { drop(Box::from_raw(ptr.as_ptr())) }
    }
    env_state.packages = NonNull::new(Box::into_raw(packages) as *mut _);
    let packages = unsafe { &*env_state.packages.unwrap().as_ptr() };
    let version = env_state.environment.environment_version.0;

    Ok(packages
        .iter()
        .enumerate()
        .map(|(index, value)| TauriPackage::new(version, index, value))
        .collect::<Vec<_>>())
}

#[derive(Serialize, specta::Type)]
struct TauriUserRepository {
    id: String,
    url: Option<String>,
    display_name: String,
}

#[derive(Serialize, specta::Type)]
pub struct TauriRepositoriesInfo {
    user_repositories: Vec<TauriUserRepository>,
    hidden_user_repositories: Vec<String>,
    hide_local_user_packages: bool,
    show_prerelease_packages: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn environment_repositories_info(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<TauriRepositoriesInfo, RustError> {
    with_environment!(&state, |environment, config| {
        Ok(TauriRepositoriesInfo {
            user_repositories: environment
                .get_user_repos()
                .iter()
                .map(|x| {
                    let id = x.id().or(x.url().map(Url::as_str)).unwrap();
                    TauriUserRepository {
                        id: id.to_string(),
                        url: x.url().map(|x| x.to_string()),
                        display_name: x.name().unwrap_or(id).to_string(),
                    }
                })
                .collect(),
            hidden_user_repositories: config.gui_hidden_repositories.iter().cloned().collect(),
            hide_local_user_packages: config.hide_local_user_packages,
            show_prerelease_packages: environment.show_prerelease_packages(),
        })
    })
}

#[tauri::command]
#[specta::specta]
pub async fn environment_hide_repository(
    state: State<'_, Mutex<EnvironmentState>>,
    repository: String,
) -> Result<(), RustError> {
    with_config!(&state, |mut config| {
        config.gui_hidden_repositories.insert(repository);
        config.save().await?;
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub async fn environment_show_repository(
    state: State<'_, Mutex<EnvironmentState>>,
    repository: String,
) -> Result<(), RustError> {
    with_config!(&state, |mut config| {
        config.gui_hidden_repositories.shift_remove(&repository);
        config.save().await?;
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub async fn environment_set_hide_local_user_packages(
    state: State<'_, Mutex<EnvironmentState>>,
    value: bool,
) -> Result<(), RustError> {
    with_environment!(&state, |_, mut config| {
        config.hide_local_user_packages = value;
        config.save().await?;
        Ok(())
    })
}

#[derive(Serialize, specta::Type)]
pub struct TauriRemoteRepositoryInfo {
    display_name: String,
    id: String,
    url: String,
    packages: Vec<TauriBasePackageInfo>,
}

#[derive(Serialize, specta::Type)]
#[serde(tag = "type")]
pub enum TauriDownloadRepository {
    BadUrl,
    Duplicated,
    DownloadError { message: String },
    Success { value: TauriRemoteRepositoryInfo },
}

// workaround IndexMap v2 is not implemented in specta

#[tauri::command]
#[specta::specta]
pub async fn environment_download_repository(
    state: State<'_, Mutex<EnvironmentState>>,
    url: String,
    headers: IndexMapV2<Box<str>, Box<str>>,
) -> Result<TauriDownloadRepository, RustError> {
    let url: Url = match url.parse() {
        Err(_) => {
            return Ok(TauriDownloadRepository::BadUrl);
        }
        Ok(url) => url,
    };

    with_environment!(state, |environment| {
        for repo in environment.get_user_repos() {
            if repo.url().map(|x| x.as_str()) == Some(url.as_str()) {
                return Ok(TauriDownloadRepository::Duplicated);
            }
        }

        let client = environment.http().unwrap();
        let repo = match RemoteRepository::download(client, &url, &headers.0).await {
            Ok((repo, _)) => repo,
            Err(e) => {
                return Ok(TauriDownloadRepository::DownloadError {
                    message: e.to_string(),
                });
            }
        };

        let url = repo.url().unwrap_or(&url).as_str();
        let id = repo.id().unwrap_or(url);

        for repo in environment.get_user_repos() {
            if repo.id() == Some(id) {
                return Ok(TauriDownloadRepository::Duplicated);
            }
        }

        Ok(TauriDownloadRepository::Success {
            value: TauriRemoteRepositoryInfo {
                id: id.to_string(),
                url: url.to_string(),
                display_name: repo.name().unwrap_or(id).to_string(),
                packages: repo
                    .get_packages()
                    .filter_map(|x| x.get_latest(VersionSelector::latest_for(None, true)))
                    .filter(|x| !x.is_yanked())
                    .map(TauriBasePackageInfo::new)
                    .collect(),
            },
        })
    })
}

#[derive(Serialize, specta::Type)]
pub enum TauriAddRepositoryResult {
    BadUrl,
    Success,
}

#[tauri::command]
#[specta::specta]
pub async fn environment_add_repository(
    state: State<'_, Mutex<EnvironmentState>>,
    url: String,
    headers: IndexMapV2<Box<str>, Box<str>>,
) -> Result<TauriAddRepositoryResult, RustError> {
    let url: Url = match url.parse() {
        Err(_) => {
            return Ok(TauriAddRepositoryResult::BadUrl);
        }
        Ok(url) => url,
    };

    with_environment!(&state, |environment| {
        environment.add_remote_repo(url, None, headers.0).await?;
        environment.save().await?;
    });

    // force update repository
    let mut state = state.lock().await;
    state.environment.last_repository_update = None;

    Ok(TauriAddRepositoryResult::Success)
}

#[tauri::command]
#[specta::specta]
pub async fn environment_remove_repository(
    state: State<'_, Mutex<EnvironmentState>>,
    id: String,
) -> Result<(), RustError> {
    with_environment!(state, |environment| {
        environment
            .remove_repo(|r| r.id() == Some(id.as_str()))
            .await;

        environment.save().await?;
    });

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_clear_package_cache(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<(), RustError> {
    with_environment!(state, |environment| {
        environment.clear_package_cache().await?;

        environment.save().await?;
    });

    Ok(())
}

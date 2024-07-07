use futures::future::try_join_all;
use indexmap::IndexMap;
use log::info;
use std::collections::HashSet;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};
use tauri::api::dialog::blocking::FileDialogBuilder;
use tauri::{Manager, State, Window};
use tokio::fs::write;
use tokio::sync::Mutex;
use url::Url;

use crate::commands::async_command::{async_command, AsyncCallResult, With};
use vrc_get_vpm::environment::AddUserPackageResult;
use vrc_get_vpm::repositories_file::RepositoriesFile;
use vrc_get_vpm::repository::RemoteRepository;
use vrc_get_vpm::{HttpClient, PackageCollection, PackageInfo, VersionSelector};

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

#[derive(Serialize, specta::Type, Clone)]
pub struct TauriRemoteRepositoryInfo {
    display_name: String,
    id: String,
    url: String,
    packages: Vec<TauriBasePackageInfo>,
}

#[derive(Serialize, specta::Type, Clone)]
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
        let user_repo_urls = user_repo_urls(environment);
        let user_repo_ids = user_repo_ids(environment);

        Ok(download_one_repository(
            environment.http().unwrap(),
            &url,
            &headers.0,
            &user_repo_urls,
            &user_repo_ids,
        )
        .await?)
    })
}

fn user_repo_urls(environment: &Environment) -> HashSet<String> {
    let mut user_repo_urls = environment
        .get_user_repos()
        .iter()
        .flat_map(|x| x.url())
        .map(|x| x.to_string())
        .collect::<HashSet<_>>();

    if !environment.ignore_curated_repository() {
        // should we check more urls?
        user_repo_urls.insert("https://packages.vrchat.com/curated?download".to_owned());
    }

    if !environment.ignore_official_repository() {
        user_repo_urls.insert("https://packages.vrchat.com/official?download".to_owned());
    }

    user_repo_urls
}

fn user_repo_ids(environment: &Environment) -> HashSet<String> {
    let mut user_repo_ids = environment
        .get_user_repos()
        .iter()
        .flat_map(|x| x.id())
        .map(|x| x.to_string())
        .collect::<HashSet<_>>();

    if !environment.ignore_curated_repository() {
        user_repo_ids.insert("com.vrchat.repos.curated".to_owned());
    }

    if !environment.ignore_official_repository() {
        user_repo_ids.insert("com.vrchat.repos.official".to_owned());
    }

    user_repo_ids
}

async fn download_one_repository(
    client: &impl HttpClient,
    repository_url: &Url,
    headers: &IndexMap<Box<str>, Box<str>>,
    user_repo_urls: &HashSet<String>,
    user_repo_ids: &HashSet<String>,
) -> Result<TauriDownloadRepository, RustError> {
    if user_repo_urls.contains(repository_url.as_str()) {
        return Ok(TauriDownloadRepository::Duplicated);
    }

    let repo = match RemoteRepository::download(client, repository_url, headers).await {
        Ok((repo, _)) => repo,
        Err(e) => {
            return Ok(TauriDownloadRepository::DownloadError {
                message: e.to_string(),
            });
        }
    };

    let url = repo.url().unwrap_or(repository_url).as_str();
    let id = repo.id().unwrap_or(url);

    if user_repo_ids.contains(id) {
        return Ok(TauriDownloadRepository::Duplicated);
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

#[derive(Serialize, specta::Type)]
#[serde(tag = "type")]
pub enum TauriImportRepositoryPickResult {
    NoFilePicked,
    ParsedRepositories {
        repositories: Vec<TauriRepositoryDescriptor>,
        unparsable_lines: Vec<String>,
    },
}

// workaround bug in specta::Type derive macro
type Headers = IndexMapV2<Box<str>, Box<str>>;

#[derive(Serialize, Deserialize, specta::Type, Clone)]
pub struct TauriRepositoryDescriptor {
    pub url: Url,
    pub headers: Headers,
}

#[tauri::command]
#[specta::specta]
pub async fn environment_import_repository_pick(
) -> Result<TauriImportRepositoryPickResult, RustError> {
    let builder = FileDialogBuilder::new();

    let Some(repositories_path) = builder.pick_file() else {
        return Ok(TauriImportRepositoryPickResult::NoFilePicked);
    };

    let repositories_file = tokio::fs::read_to_string(repositories_path).await?;

    let result = RepositoriesFile::parse(&repositories_file);

    Ok(TauriImportRepositoryPickResult::ParsedRepositories {
        repositories: result
            .parsed()
            .repositories()
            .iter()
            .map(|x| TauriRepositoryDescriptor {
                url: x.url().clone(),
                headers: IndexMapV2(x.headers().clone()),
            })
            .collect(),
        unparsable_lines: result.unparseable_lines().to_vec(),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn environment_import_download_repositories(
    window: Window,
    channel: String,
    repositories: Vec<TauriRepositoryDescriptor>,
) -> Result<
    AsyncCallResult<usize, Vec<(TauriRepositoryDescriptor, TauriDownloadRepository)>>,
    RustError,
> {
    async_command(channel, window.clone(), async move {
        With::<usize>::continue_async(|ctx| async move {
            let state = window.state::<Mutex<EnvironmentState>>();
            with_environment!(state, |environment| {
                let user_repo_urls = user_repo_urls(environment);
                let mut user_repo_ids = user_repo_ids(environment);

                info!("downloading {} repositories", repositories.len());

                let client = environment.http().unwrap();

                let counter = AtomicUsize::new(0);

                let counter_ref = &counter;
                let user_repo_urls_ref = &user_repo_urls;
                let user_repo_ids_ref = &user_repo_ids;

                let mut results = try_join_all(repositories.into_iter().map(|adding_repo| {
                    let ctx = ctx.clone();
                    async move {
                        let downloaded = download_one_repository(
                            client,
                            &adding_repo.url,
                            &adding_repo.headers.0,
                            user_repo_urls_ref,
                            user_repo_ids_ref,
                        )
                        .await?;

                        info!("downloaded repository: {:?}", adding_repo.url);

                        let count = counter_ref.fetch_add(1, Ordering::Relaxed);
                        ctx.emit(count).unwrap();

                        Ok::<_, RustError>((adding_repo, downloaded))
                    }
                }))
                .await?;

                for (_, downloaded) in results.as_mut_slice() {
                    if let TauriDownloadRepository::Success { value } = &downloaded {
                        if user_repo_ids.contains(&value.id) {
                            info!("duplicated repository in list: {}", value.url);
                            *downloaded = TauriDownloadRepository::Duplicated;
                        } else {
                            user_repo_ids.insert(value.id.to_string());
                        }
                    }
                }

                Ok(results)
            })
        })
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn environment_import_add_repositories(
    state: State<'_, Mutex<EnvironmentState>>,
    repositories: Vec<TauriRepositoryDescriptor>,
) -> Result<(), RustError> {
    with_environment!(&state, |environment| {
        for adding_repo in repositories {
            environment
                .add_remote_repo(adding_repo.url, None, adding_repo.headers.0)
                .await?;
        }
        environment.save().await?;
    });

    // force update repository
    let mut state = state.lock().await;
    state.environment.last_repository_update = None;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn environment_export_repositories(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<(), RustError> {
    let Some(path) = FileDialogBuilder::new()
        .add_filter("Text", &["txt"])
        .set_file_name("repositories")
        .save_file()
    else {
        return Ok(());
    };

    let repositories = with_environment!(state, |environment| environment.export_repositories());

    write(path, repositories).await?;

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

#[derive(Serialize, specta::Type)]
pub struct TauriUserPackage {
    path: String,
    package: TauriBasePackageInfo,
}

#[tauri::command]
#[specta::specta]
pub async fn environment_get_user_packages(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<Vec<TauriUserPackage>, RustError> {
    let mut env_state = state.lock().await;
    let env_state = &mut *env_state;
    let environment = env_state
        .environment
        .get_environment_mut(
            UpdateRepositoryMode::IfOutdatedOrNecessaryForLocal,
            &env_state.io,
        )
        .await?;

    Ok(environment
        .user_packages()
        .iter()
        .filter_map(|(path, json)| {
            let path = path.as_os_str().to_str()?;
            Some(TauriUserPackage {
                path: path.into(),
                package: TauriBasePackageInfo::new(json),
            })
        })
        .collect())
}

#[derive(Serialize, specta::Type)]
pub enum TauriAddUserPackageWithPickerResult {
    NoFolderSelected,
    InvalidSelection,
    AlreadyAdded,
    Successful,
}

#[tauri::command]
#[specta::specta]
pub async fn environment_add_user_package_with_picker(
    state: State<'_, Mutex<EnvironmentState>>,
) -> Result<TauriAddUserPackageWithPickerResult, RustError> {
    let Some(project_path) = FileDialogBuilder::new().pick_folder() else {
        return Ok(TauriAddUserPackageWithPickerResult::NoFolderSelected);
    };

    let Ok(project_path) = project_path.into_os_string().into_string() else {
        return Ok(TauriAddUserPackageWithPickerResult::InvalidSelection);
    };

    with_environment!(&state, |environment| {
        match environment.add_user_package(project_path.as_ref()).await {
            AddUserPackageResult::Success => {}
            AddUserPackageResult::NonAbsolute => unreachable!("absolute path"),
            AddUserPackageResult::BadPackage => {
                return Ok(TauriAddUserPackageWithPickerResult::InvalidSelection);
            }
            AddUserPackageResult::AlreadyAdded => {
                return Ok(TauriAddUserPackageWithPickerResult::AlreadyAdded);
            }
        }

        environment.save().await?;
    });

    Ok(TauriAddUserPackageWithPickerResult::Successful)
}

#[tauri::command]
#[specta::specta]
pub async fn environment_remove_user_packages(
    state: State<'_, Mutex<EnvironmentState>>,
    path: String,
) -> Result<(), RustError> {
    with_environment!(state, |environment| {
        environment.remove_user_package(Path::new(&path));

        environment.save().await?;
    });

    {
        let mut state = state.lock().await;
        state.environment.last_repository_update = None;
    }

    Ok(())
}

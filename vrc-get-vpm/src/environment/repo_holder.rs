use crate::environment::repo_source::RepoSource;
use crate::environment::{
    CURATED_URL_STR, LOCAL_CURATED_PATH, LOCAL_OFFICIAL_PATH, OFFICIAL_URL_STR, Settings,
};
use crate::io::{DefaultEnvironmentIo, IoTrait};
use crate::repository::RemoteRepository;
use crate::repository::local::LocalCachedRepository;
use crate::traits::HttpClient;
use crate::utils::{read_json_file, to_vec_pretty_os_eol, try_load_json};
use crate::{UserRepoSetting, io};
use futures::future::join_all;
use indexmap::IndexMap;
use lazy_static::lazy_static;
use log::{error, warn};
use std::collections::HashMap;
use std::path::Path;
use url::Url;

#[derive(Debug, Clone)]
pub(crate) struct RepoHolder {
    cached_repos_new: HashMap<Box<Path>, Repository>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
enum Repository {
    Loaded(LocalCachedRepository),
    NotDownloaded(Url, IndexMap<Box<str>, Box<str>>),
    UnableToLoad,
}

impl Repository {
    fn as_loaded(&self) -> Option<&LocalCachedRepository> {
        match self {
            Repository::Loaded(x) => Some(x),
            _ => None,
        }
    }

    fn remote_download_info(&self) -> Option<RemoteDownloadInfo<'_>> {
        match self {
            Repository::Loaded(repo) => repo.url().map(|url| RemoteDownloadInfo {
                url,
                headers: repo.headers(),
                etag: repo.vrc_get.as_ref().map(|x| x.etag.as_ref()),
            }),
            Repository::NotDownloaded(url, headers) => Some(RemoteDownloadInfo {
                url,
                headers,
                etag: None,
            }),
            Repository::UnableToLoad => None,
        }
    }
}

struct RemoteDownloadInfo<'a> {
    url: &'a Url,
    headers: &'a IndexMap<Box<str>, Box<str>>,
    etag: Option<&'a str>,
}

impl RepoHolder {
    pub(crate) fn new() -> Self {
        RepoHolder {
            cached_repos_new: HashMap::new(),
        }
    }
}

/// accessors
impl RepoHolder {
    pub fn remove(&mut self, path: &Path) {
        self.cached_repos_new.remove(path);
    }

    pub fn iter(&self) -> impl Iterator<Item = &LocalCachedRepository> + Sized {
        self.cached_repos_new.values().filter_map(|x| x.as_loaded())
    }

    pub fn find_by_id(&self, id: &str) -> Option<&LocalCachedRepository> {
        self.iter().find(|x| x.id() == Some(id))
    }

    pub fn get_by_path(&self, path: &Path) -> Option<&LocalCachedRepository> {
        self.cached_repos_new.get(path).and_then(|x| x.as_loaded())
    }
}

// new system
impl RepoHolder {
    pub(crate) async fn load(
        settings: &Settings,
        io: &DefaultEnvironmentIo,
        http: Option<&impl HttpClient>,
    ) -> io::Result<Self> {
        let mut repo_cache = Self::load_cache(settings, io).await?;

        if let Some(http) = http {
            repo_cache.update_cache(io, http).await;
        }

        Ok(repo_cache)
    }

    pub(crate) async fn load_cache(
        settings: &Settings,
        io: &DefaultEnvironmentIo,
    ) -> io::Result<Self> {
        let predefined_repos = Self::get_predefined_repos(settings).into_iter();
        let user_repos = settings
            .get_user_repos()
            .iter()
            .map(UserRepoSetting::to_source);
        io.create_dir_all("Repos".as_ref()).await?;
        let mut repo_cache = Self::new();

        repo_cache
            .load_repo_cache(io, predefined_repos.chain(user_repos))
            .await;

        Ok(repo_cache)
    }

    fn get_predefined_repos(settings: &Settings) -> Vec<RepoSource<'static>> {
        lazy_static! {
            static ref EMPTY_HEADERS: IndexMap<Box<str>, Box<str>> = IndexMap::new();
            static ref OFFICIAL_URL: Url = Url::parse(OFFICIAL_URL_STR).unwrap();
            static ref CURATED_URL: Url = Url::parse(CURATED_URL_STR).unwrap();
        }

        let mut repositories = Vec::with_capacity(2);

        if !settings.ignore_official_repository() {
            repositories.push(RepoSource::new(
                LOCAL_OFFICIAL_PATH.as_ref(),
                &EMPTY_HEADERS,
                Some(&OFFICIAL_URL),
            ));
        } else {
            warn!("ignoring official repository is experimental feature!");
        }

        if !settings.ignore_curated_repository() {
            repositories.push(RepoSource::new(
                LOCAL_CURATED_PATH.as_ref(),
                &EMPTY_HEADERS,
                Some(&CURATED_URL),
            ));
        } else {
            warn!("ignoring curated repository is experimental feature!");
        }

        repositories
    }

    /// Note: errors will be logged instead of returning
    pub(crate) async fn load_repo_cache<'a>(
        &mut self,
        io: &DefaultEnvironmentIo,
        sources: impl Iterator<Item = RepoSource<'a>>,
    ) {
        let start = std::time::Instant::now();
        let repos = join_all(sources.map(|src| async move {
            fn if_not_exists(src: RepoSource) -> (Box<Path>, Repository) {
                (
                    src.cache_path().into(),
                    src.url()
                        .map(|u| Repository::NotDownloaded(u.clone(), src.headers().clone()))
                        .unwrap_or(Repository::UnableToLoad),
                )
            }

            match Self::load_repo_from_cache(io, &src).await {
                Ok(Some(v)) => (src.cache_path().into(), Repository::Loaded(v)),
                Ok(None) => if_not_exists(src),
                Err(e) => {
                    error!("loading repo '{}': {e}", src.cache_path().display());
                    if_not_exists(src)
                }
            }
        }))
        .await;
        let duration = std::time::Instant::now() - start;
        log::debug!("loading repo cache took {duration:?}");

        for (path, repo) in repos.into_iter() {
            self.cached_repos_new.insert(path, repo);
        }
    }

    async fn load_repo_from_cache(
        io: &DefaultEnvironmentIo,
        source: &RepoSource<'_>,
    ) -> io::Result<Option<LocalCachedRepository>> {
        let path = source.cache_path();
        if let Some(url) = source.url() {
            if let Some(mut loaded) = try_load_json::<LocalCachedRepository>(io, path).await? {
                loaded.set_url(url.clone());
                Ok(Some(loaded))
            } else {
                warn!("Local cache for {url} does not exist");
                Ok(None)
            }
        } else {
            Ok(Some(
                read_json_file::<LocalCachedRepository>(io.open(path).await?, path).await?,
            ))
        }
    }

    pub(crate) async fn update_cache(
        &mut self,
        io: &DefaultEnvironmentIo,
        client: &impl HttpClient,
    ) {
        let start = std::time::Instant::now();
        let result = futures::future::join_all(self.cached_repos_new.iter_mut().map(
            async |(path, repository)| {
                if let Some(info) = repository.remote_download_info() {
                    log::debug!("downloading remote repo '{}'", info.url);
                    match RemoteRepository::download_with_etag(
                        client,
                        info.url,
                        info.headers,
                        info.etag,
                    )
                    .await
                    {
                        Ok(Some((remote_repo, etag))) => {
                            log::debug!("successfully downloaded '{}'", info.url);

                            let headers = info.headers.clone();
                            let new_repository = if let Repository::Loaded(existing) = repository {
                                existing.set_repo(remote_repo);
                                existing
                            } else {
                                //let headers = headers.clone(); // lifetime error
                                *repository = Repository::Loaded(LocalCachedRepository::new(
                                    remote_repo,
                                    headers,
                                ));
                                match repository {
                                    Repository::Loaded(x) => x,
                                    _ => unreachable!(),
                                }
                            };

                            new_repository.set_etag(etag);

                            async fn save_repository(
                                io: &DefaultEnvironmentIo,
                                path: &Path,
                                repository: &LocalCachedRepository,
                            ) -> io::Result<()> {
                                io.write_sync(path, &to_vec_pretty_os_eol(&repository)?)
                                    .await
                            }

                            if let Err(e) = save_repository(io, path, new_repository).await {
                                error!("writing local repo cache '{}': {e}", path.display());
                            }
                        }
                        Ok(None) => {
                            log::debug!("already up to date, using cached '{}'", info.url)
                        }
                        // error handling later
                        Err(e) => return Err((info.url.clone(), e)),
                    }

                    Ok(true)
                } else {
                    // */
                    Ok(false)
                }
            },
        ))
        .await;

        log::debug!("updating repo from remote took {:?}", start.elapsed());

        handle_error(result);

        fn handle_error(result: Vec<Result<bool, (Url, io::Error)>>) {
            // We want to workaround 'Connection Refused' spam on offline environment,
            // so if all repositories reported error,
            // we report single "Unable to connect to any servers".

            if result.is_empty() || result.iter().any(|x| x.is_ok()) {
                // some succeeded, so normal error handling
                return log_error(result);
            }

            error!("fetching remote repo: Unable to download from servers");
        }

        fn log_error(result: Vec<Result<bool, (Url, io::Error)>>) {
            for result in result {
                if let Some((url, error)) = result.err() {
                    error!("fetching remote repo '{url}': {error}");
                }
            }
        }
    }
}

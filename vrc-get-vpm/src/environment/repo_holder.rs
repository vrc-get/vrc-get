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
    cached_repos_new: HashMap<Box<Path>, LocalCachedRepository>,
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
        self.cached_repos_new.values()
    }

    pub fn find_by_id(&self, id: &str) -> Option<&LocalCachedRepository> {
        self.cached_repos_new.values().find(|x| x.id() == Some(id))
    }

    pub fn get_by_path(&self, path: &Path) -> Option<&LocalCachedRepository> {
        self.cached_repos_new.get(path)
    }
}

// new system
impl RepoHolder {
    pub(crate) async fn load(
        settings: &Settings,
        io: &DefaultEnvironmentIo,
        http: Option<&impl HttpClient>,
    ) -> io::Result<Self> {
        let predefined_repos = Self::get_predefined_repos(settings).into_iter();
        let user_repos = settings
            .get_user_repos()
            .iter()
            .map(UserRepoSetting::to_source);
        io.create_dir_all("Repos".as_ref()).await?;
        let mut repo_cache = Self::new();
        repo_cache
            .load_repos(http, io, predefined_repos.chain(user_repos))
            .await?;

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

    pub(crate) async fn load_repos<'a>(
        &mut self,
        http: Option<&impl HttpClient>,
        io: &DefaultEnvironmentIo,
        sources: impl Iterator<Item = RepoSource<'a>>,
    ) -> io::Result<()> {
        let start = std::time::Instant::now();
        let repos = join_all(sources.map(|src| async move {
            match Self::load_repo_from_source(http, io, &src).await {
                Ok(Some(v)) => Some((v, src.cache_path().into())),
                Ok(None) => None,
                Err(e) => {
                    error!("loading repo '{}': {}", src.cache_path().display(), e);
                    None
                }
            }
        }))
        .await;
        let duration = std::time::Instant::now() - start;
        log::info!("downloading repos took {:?}", duration);

        for (repo, path) in repos.into_iter().flatten() {
            self.cached_repos_new.insert(path, repo);
        }

        Ok(())
    }

    async fn load_repo_from_source(
        client: Option<&impl HttpClient>,
        io: &DefaultEnvironmentIo,
        source: &RepoSource<'_>,
    ) -> io::Result<Option<LocalCachedRepository>> {
        if let Some(url) = &source.url() {
            RepoHolder::load_remote_repo(client, io, source.headers(), source.cache_path(), url)
                .await
                .map(Some)
        } else {
            RepoHolder::load_local_repo(io, source.cache_path())
                .await
                .map(Some)
        }
    }

    async fn load_remote_repo(
        client: Option<&impl HttpClient>,
        io: &DefaultEnvironmentIo,
        headers: &IndexMap<Box<str>, Box<str>>,
        path: &Path,
        remote_url: &Url,
    ) -> io::Result<LocalCachedRepository> {
        if let Some(mut loaded) = try_load_json::<LocalCachedRepository>(io, path).await? {
            if let Some(client) = client {
                // if it's possible to download remote repo, try to update with that
                log::debug!("downloading remote repo '{}' with local cache", remote_url);
                match RemoteRepository::download_with_etag(
                    client,
                    remote_url,
                    loaded.headers(),
                    loaded.vrc_get.as_ref().map(|x| x.etag.as_ref()),
                )
                .await
                {
                    Ok(None) => log::debug!("cache matched downloading '{}'", remote_url),
                    Ok(Some((remote_repo, etag))) => {
                        log::debug!("downloaded finished '{}'", remote_url);
                        loaded.set_repo(remote_repo);
                        loaded.set_etag(etag);

                        io.write(path, &to_vec_pretty_os_eol(&loaded)?)
                            .await
                            .unwrap_or_else(|e| {
                                error!("writing local repo cache '{}': {}", path.display(), e)
                            });
                    }
                    Err(e) => {
                        error!("fetching remote repo '{}': {}", remote_url, e);
                    }
                }
            }

            Ok(loaded)
        } else {
            // if local repository not found: try downloading remote one
            let Some(client) = client else {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "offline mode",
                ));
            };
            log::debug!("downloading remote repo '{}'", remote_url);
            let (remote_repo, etag) =
                RemoteRepository::download(client, remote_url, headers).await?;
            log::debug!("downloaded finished '{}'", remote_url);

            let mut local_cache = LocalCachedRepository::new(remote_repo, headers.clone());

            local_cache.set_etag(etag);

            io.write(path, &to_vec_pretty_os_eol(&local_cache)?)
                .await
                .unwrap_or_else(|e| {
                    error!("writing local repo cache '{}': {}", path.display(), e);
                });

            Ok(local_cache)
        }
    }

    async fn load_local_repo(
        io: &DefaultEnvironmentIo,
        path: &Path,
    ) -> io::Result<LocalCachedRepository> {
        read_json_file::<LocalCachedRepository>(io.open(path).await?, path).await
    }
}

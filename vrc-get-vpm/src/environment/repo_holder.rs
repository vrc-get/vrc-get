use crate::environment::repo_source::RepoSource;
use crate::io;
use crate::io::EnvironmentIo;
use crate::repository::local::LocalCachedRepository;
use crate::repository::RemoteRepository;
use crate::traits::HttpClient;
use crate::utils::{read_json_file, to_vec_pretty_os_eol, try_load_json};
use futures::future::join_all;
use indexmap::IndexMap;
use log::error;
use std::collections::HashMap;
use std::path::Path;
use url::Url;

#[derive(Debug)]
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

// new system
impl RepoHolder {
    pub(crate) async fn load_repos<'a, IO: EnvironmentIo>(
        &mut self,
        http: Option<&impl HttpClient>,
        io: &IO,
        sources: impl Iterator<Item = RepoSource<'a>>,
    ) -> io::Result<()> {
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

        for (repo, path) in repos.into_iter().flatten() {
            self.cached_repos_new.insert(path, repo);
        }

        Ok(())
    }

    async fn load_repo_from_source<IO: EnvironmentIo>(
        client: Option<&impl HttpClient>,
        io: &IO,
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
        io: &impl EnvironmentIo,
        headers: &IndexMap<Box<str>, Box<str>>,
        path: &Path,
        remote_url: &Url,
    ) -> io::Result<LocalCachedRepository> {
        if let Some(mut loaded) = try_load_json::<LocalCachedRepository>(io, path).await? {
            if let Some(client) = client {
                // if it's possible to download remote repo, try to update with that
                match RemoteRepository::download_with_etag(
                    client,
                    remote_url,
                    loaded.headers(),
                    loaded.vrc_get.as_ref().map(|x| x.etag.as_ref()),
                )
                .await
                {
                    Ok(None) => log::debug!("cache matched downloading {}", remote_url),
                    Ok(Some((remote_repo, etag))) => {
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
            let (remote_repo, etag) =
                RemoteRepository::download(client, remote_url, headers).await?;

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
        io: &impl EnvironmentIo,
        path: &Path,
    ) -> io::Result<LocalCachedRepository> {
        read_json_file::<LocalCachedRepository>(io.open(path).await?, path).await
    }

    pub(crate) fn get_repos(&self) -> Vec<&LocalCachedRepository> {
        self.cached_repos_new.values().collect()
    }

    pub(crate) fn get_repo(&self, path: &Path) -> Option<&LocalCachedRepository> {
        self.cached_repos_new.get(path)
    }

    pub(crate) fn remove_repo(&mut self, path: &Path) {
        self.cached_repos_new.remove(path);
    }
}

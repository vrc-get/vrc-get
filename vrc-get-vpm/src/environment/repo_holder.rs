use super::*;
use crate::environment::repo_source::RepoSource;
use crate::traits::HttpClient;
use crate::{read_to_vec, try_open_file, update_from_remote, write_repo};
use futures::future::try_join_all;
use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use url::Url;

#[derive(Debug)]
pub(crate) struct RepoHolder {
    cached_repos_new: HashMap<PathBuf, LocalCachedRepository>,
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
    pub(crate) async fn load_repos(
        &mut self,
        http: Option<&impl HttpClient>,
        sources: impl Iterator<Item = impl RepoSource>,
    ) -> io::Result<()> {
        let repos = try_join_all(sources.map(|src| async move {
            Self::load_repo_from_source(http, &src)
                .await
                .map(|v| v.map(|v| (v, src.cache_path().to_path_buf())))
        }))
        .await?;

        for (repo, path) in repos.into_iter().flatten() {
            self.cached_repos_new.insert(path, repo);
        }

        Ok(())
    }

    async fn load_repo_from_source(
        client: Option<&impl HttpClient>,
        source: &impl RepoSource,
    ) -> io::Result<Option<LocalCachedRepository>> {
        if let Some(url) = &source.url() {
            RepoHolder::load_remote_repo(client, source.headers(), source.cache_path(), url)
                .await
                .map(Some)
        } else {
            RepoHolder::load_local_repo(source.cache_path())
                .await
                .map(Some)
        }
    }

    async fn load_remote_repo(
        client: Option<&impl HttpClient>,
        headers: &IndexMap<String, String>,
        path: &Path,
        remote_url: &Url,
    ) -> io::Result<LocalCachedRepository> {
        Self::load_repo(path, client, || async {
            // if local repository not found: try downloading remote one
            let Some(client) = client else {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "offline mode",
                ));
            };
            let (remote_repo, etag) =
                RemoteRepository::download_with_etag(client, remote_url, headers, None)
                    .await?
                    .expect("logic failure: no etag");

            let mut local_cache = LocalCachedRepository::new(remote_repo, headers.clone());

            if let Some(etag) = etag {
                local_cache
                    .vrc_get
                    .get_or_insert_with(Default::default)
                    .etag = etag;
            }

            match write_repo(path, &local_cache).await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("writing local repo '{}': {}", path.display(), e);
                }
            }

            Ok(local_cache)
        })
        .await
    }

    async fn load_local_repo(path: &Path) -> io::Result<LocalCachedRepository> {
        Self::load_repo(path, Option::<&Infallible>::None, || async {
            unreachable!()
        })
        .await
    }

    async fn load_repo<F, T>(
        path: &Path,
        http: Option<&impl HttpClient>,
        if_not_found: F,
    ) -> io::Result<LocalCachedRepository>
    where
        F: FnOnce() -> T,
        T: Future<Output = io::Result<LocalCachedRepository>>,
    {
        let Some(json_file) = try_open_file(path).await? else {
            return if_not_found().await;
        };

        let mut loaded = match serde_json::from_slice(&read_to_vec(json_file).await?) {
            Ok(loaded) => loaded,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("loading {}: {}", path.display(), e),
                ))
            }
        };
        if let Some(http) = http {
            update_from_remote(http, path, &mut loaded).await;
        }
        Ok(loaded)
    }

    pub(crate) fn get_repos(&self) -> Vec<&LocalCachedRepository> {
        self.cached_repos_new.values().collect()
    }

    pub(crate) fn get_repo_with_path(
        &self,
    ) -> impl Iterator<Item = (&'_ PathBuf, &'_ LocalCachedRepository)> {
        self.cached_repos_new.iter()
    }

    pub(crate) fn get_repo(&self, path: &Path) -> Option<&LocalCachedRepository> {
        self.cached_repos_new.get(path)
    }

    pub(crate) fn remove_repo(&mut self, path: &Path) {
        self.cached_repos_new.remove(path);
    }
}

impl PackageCollection for RepoHolder {
    fn get_all_packages(&self) -> impl Iterator<Item = PackageInfo> {
        self.get_repos()
            .into_iter()
            .flat_map(|repo| repo.get_all_packages())
    }

    fn find_packages(&self, package: &str) -> impl Iterator<Item = PackageInfo> {
        self.get_repos()
            .into_iter()
            .flat_map(|repo| repo.find_packages(package))
    }

    fn find_package_by_name(
        &self,
        package: &str,
        package_selector: VersionSelector,
    ) -> Option<PackageInfo> {
        self.get_repos()
            .into_iter()
            .flat_map(|repo| repo.find_package_by_name(package, package_selector))
            .max_by_key(|x| x.version())
    }
}

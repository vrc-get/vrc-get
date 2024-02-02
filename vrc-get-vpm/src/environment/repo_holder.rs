use super::*;
use crate::environment::repo_source::RepoSource;
use crate::traits::HttpClient;
use crate::utils::{read_json_file, try_load_json};
use futures::future::try_join_all;
use std::collections::HashMap;
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
        if let Some(mut loaded) = try_load_json::<LocalCachedRepository>(path).await? {
            if let (Some(client), Some(remote_url)) = (client, loaded.url().map(|x| x.to_owned())) {
                // if it's possible to download remote repo, try to update with that
                match RemoteRepository::download_with_etag(
                    client,
                    &remote_url,
                    loaded.headers(),
                    loaded.vrc_get.as_ref().map(|x| x.etag.as_str()),
                )
                .await
                {
                    Ok(None) => log::debug!("cache matched downloading {}", remote_url),
                    Ok(Some((remote_repo, etag))) => {
                        loaded.set_repo(remote_repo);
                        loaded.set_etag(etag);

                        tokio::fs::write(path, &to_vec_pretty_os_eol(&loaded)?)
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

            tokio::fs::write(path, &to_vec_pretty_os_eol(&local_cache)?)
                .await
                .unwrap_or_else(|e| {
                    error!("writing local repo cache '{}': {}", path.display(), e);
                });

            Ok(local_cache)
        }
    }

    async fn load_local_repo(path: &Path) -> io::Result<LocalCachedRepository> {
        read_json_file::<LocalCachedRepository>(File::open(path).await?, path).await
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

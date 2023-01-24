use super::*;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::future::Future;

#[derive(Debug)]
pub(super) struct RepoHolder {
    http: Client,
    // the pointer of LocalCachedRepository will never be changed
    cached_repos: UnsafeCell<HashMap<PathBuf, Box<LocalCachedRepository>>>,
}

impl RepoHolder {
    pub(crate) fn new(http: Client) -> Self {
        RepoHolder {
            http,
            cached_repos: UnsafeCell::new(HashMap::new()),
        }
    }

    /// Get OR create and update repository
    pub(crate) async fn get_or_create_repo(
        &self,
        path: &Path,
        remote_url: &str,
        name: Option<&str>,
    ) -> io::Result<&LocalCachedRepository> {
        let client = self.http.clone();
        self.get_repo(path, || async {
            // if local repository not found: try downloading remote one
            let remote_repo = download_remote_repository(&client, remote_url).await?;

            let mut local_cache = LocalCachedRepository::new(
                path.to_owned(),
                name.map(str::to_owned),
                Some(remote_url.to_owned()),
            );
            local_cache.cache = remote_repo
                .get("packages")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or(JsonMap::new());
            local_cache.repo = Some(remote_repo);

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

    pub(crate) async fn get_repo<F, T>(
        &self,
        path: &Path,
        if_not_found: F,
    ) -> io::Result<&LocalCachedRepository>
    where
        F: FnOnce() -> T,
        T: Future<Output = io::Result<LocalCachedRepository>>,
    {
        if let Some(found) = unsafe { (*self.cached_repos.get()).get(path) } {
            return Ok(found);
        }

        let Some(json_file) = try_open_file(path).await? else {
            let new_value = Box::new(if_not_found().await?);
            return Ok(unsafe { (*self.cached_repos.get()).entry(path.into()) }.or_insert(new_value))
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
        update_from_remote(&self.http, path.into(), &mut loaded).await;
        Ok(unsafe { (*self.cached_repos.get()).entry(path.into()) }.or_insert(Box::new(loaded)))
    }

    pub(crate) async fn get_user_repo(
        &self,
        repo: &UserRepoSetting,
    ) -> io::Result<&LocalCachedRepository> {
        if let Some(url) = &repo.url {
            self.get_or_create_repo(&repo.local_path, &url, repo.name.as_deref())
                .await
        } else {
            self.get_repo(&repo.local_path, || async {
                Err(io::Error::new(io::ErrorKind::NotFound, "repo not found"))
            })
            .await
        }
    }
}

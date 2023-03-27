use super::*;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::future::Future;
use std::marker::{PhantomData, PhantomPinned};
use std::pin::Pin;
use tokio::sync::Semaphore;

#[derive(Debug)]
pub(super) struct RepoHolder {
    http: Option<Client>,
    // the pointer of LocalCachedRepository will never be changed
    cached_repos: UnsafeCell<HashMap<PathBuf, Pin<Box<RepositoryHolder>>>>,
}

impl RepoHolder {
    pub(crate) fn new(http: Option<Client>) -> Self {
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
            let Some(client) = client else {
                return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "offline mode"))
            };
            let (remote_repo, etag) = download_remote_repository(&client, remote_url, None)
                .await?
                .expect("logic failure: no etag");

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

    fn get_repo_holder(&self, path: PathBuf) -> &RepositoryHolder {
        // SAFETY: 
        // 1) the RepositoryHolder instance is pinned 
        // 2) Also, value of cached_repos is never updated
        //
        // so RepositoryHolder reference is live if self is live
        unsafe { (*self.cached_repos.get()).entry(path).or_insert_with(|| Box::pin(RepositoryHolder::new())) }
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
        self.get_repo_holder(path.into()).get_repo(path, self.http.as_ref(), if_not_found).await
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

struct RepositoryHolder {
    // used as lock. write lock only
    semaphore: Semaphore,
    value: UnsafeCell<Option<LocalCachedRepository>>,
    // pinned, non-sync/send
    _phantom: PhantomData<PhantomPinned>,
}

impl RepositoryHolder {
    pub(super) fn new() -> Self {
        Self {
            semaphore: Semaphore::new(1),
            value: UnsafeCell::new(None),
            _phantom: PhantomData
        }
    }

    pub(super) async fn get_repo<F, T>(
        &self,
        path: &Path,
        http: Option<&Client>,
        if_not_found: F,
    ) -> io::Result<&LocalCachedRepository>
        where
            F: FnOnce() -> T,
            T: Future<Output = io::Result<LocalCachedRepository>>,
    {
        if let Some(ref found) = unsafe { &*self.value.get() } {
            return Ok(found);
        }

        let guard = self.semaphore.acquire().await.unwrap();

        if let Some(ref found) = unsafe { &*self.value.get() } {
            return Ok(found);
        }

        let Some(json_file) = try_open_file(path).await? else {
            return Ok(self.set_value(if_not_found().await?));
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
            update_from_remote(http, path.into(), &mut loaded).await;
        }
        return Ok(self.set_value(loaded));
    }

    fn set_value(&self, value: LocalCachedRepository) -> &LocalCachedRepository {
        assert!(self.semaphore.available_permits() == 0, "semaphore lock is not owned");

        unsafe { *self.value.get() = Some(value); }
        return unsafe { (*self.value.get()).as_ref().unwrap_unchecked() };
    }
}

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
    cached_repos_new: HashMap<PathBuf, LocalCachedRepository>,
}

impl RepoHolder {
    pub(crate) fn new(http: Option<Client>) -> Self {
        RepoHolder {
            http,
            cached_repos: UnsafeCell::new(HashMap::new()),
            cached_repos_new: HashMap::new(),
        }
    }
}

// new system
impl RepoHolder {
    pub(crate) async fn load_repos(&mut self, sources: Vec<RepoSource>) -> io::Result<()> {
        fn file_path(source: &RepoSource) -> &Path {
            match source {
                RepoSource::PreDefined(_, path) => path,
                RepoSource::UserRepo(user) => &user.local_path,
                RepoSource::Undefined(undefined) => undefined,
            }
        }

        let repos = try_join_all(sources.iter().map(|src| async {
            Self::load_repo_from_source(self.http.as_ref(), src).await
                .map(|v| (v, file_path(src)))
        }))
        .await?;

        for (repo, path) in repos {
            self.cached_repos_new.insert(path.to_owned(), repo);
        }

        Ok(())
    }

    async fn load_repo_from_source(
        client: Option<&Client>,
        source: &RepoSource,
    ) -> io::Result<LocalCachedRepository> {
        match source {
            RepoSource::PreDefined(source, path) => {
                RepoHolder::load_remote_repo(client, &path, source.url, Some(source.name)).await
            }
            RepoSource::UserRepo(user_repo) => {
                if let Some(url) = &user_repo.url {
                    RepoHolder::load_remote_repo(
                        client,
                        &user_repo.local_path,
                        &url,
                        user_repo.name.as_deref(),
                    )
                    .await
                } else {
                    RepoHolder::load_local_repo(client, &user_repo.local_path).await
                }
            }
            RepoSource::Undefined(repo_json) => {
                RepoHolder::load_local_repo(client, &repo_json).await
            }
        }
    }

    async fn load_remote_repo(
        client: Option<&Client>,
        path: &Path,
        remote_url: &str,
        name: Option<&str>,
    ) -> io::Result<LocalCachedRepository> {
        Self::load_repo(path, client, || async {
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

    async fn load_local_repo(
        client: Option<&Client>,
        path: &Path,
    ) -> io::Result<LocalCachedRepository> {
        Self::load_repo(path, client, || async { unreachable!() }).await
    }

    async fn load_repo<F, T>(
        path: &Path,
        http: Option<&Client>,
        if_not_found: F,
    ) -> io::Result<LocalCachedRepository>
    where
        F: FnOnce() -> T,
        T: Future<Output = io::Result<LocalCachedRepository>>,
    {
        let Some(json_file) = try_open_file(path).await? else {
            return Ok(if_not_found().await?);
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
        return Ok(loaded);
    }

    pub(crate) fn get_repo(&self, path: &Path) -> Option<&LocalCachedRepository> {
        self.cached_repos_new.get(path)
    }

    pub(crate) fn get_repos(&self) -> Vec<&LocalCachedRepository> {
        self.cached_repos_new.values().collect()
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

use crate::utils::YokeExt;
use arc_swap::ArcSwapOption;
use std::future::Future;
use std::io;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use vrc_get_vpm::environment::{PackageCollection, Settings};
use vrc_get_vpm::io::DefaultEnvironmentIo;
use vrc_get_vpm::{PackageCollection as _, PackageInfo};
use yoke::{Yoke, Yokeable};

#[derive(Yokeable)]
struct YokeData<'env> {
    packages: Vec<PackageInfo<'env>>,
}

impl<'env> YokeData<'env> {
    pub fn new(packages: Vec<PackageInfo<'env>>) -> Self {
        Self { packages }
    }
}

type Data = Yoke<YokeData<'static>, Arc<PackageCollection>>;

struct PackagesStateInner {
    pub data: Data,
    pub crated_at: std::time::Instant,
}

impl PackagesStateInner {
    pub fn new(data: Data) -> Self {
        let crated_at = std::time::Instant::now();
        Self { data, crated_at }
    }

    pub fn is_new(&self) -> bool {
        self.crated_at.elapsed() < Duration::from_secs(60 * 5)
    }
}

pub struct PackagesState {
    inner: Arc<ArcSwapOption<PackagesStateInner>>,
    load_lock: tokio::sync::Mutex<()>,
}

impl PackagesState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ArcSwapOption::new(None)),
            load_lock: tokio::sync::Mutex::new(()),
        }
    }

    pub async fn load_fully(
        &self,
        settings: &Settings,
        io: &DefaultEnvironmentIo,
        http: &reqwest::Client,
    ) -> io::Result<PackagesStateRef<'_>> {
        let inner = self.inner.load_full();

        // If the data is new enough, we can use it.
        if let Some(inner) = inner.filter(|x| x.is_new()) {
            return Ok(PackagesStateRef {
                arc: inner,
                _phantom_data: PhantomData,
            });
        }

        self.load_impl(settings, io, http, None, false).await
    }

    pub async fn load(
        &self,
        settings: &Settings,
        io: &DefaultEnvironmentIo,
        http: &reqwest::Client,
        app_handle: AppHandle,
    ) -> io::Result<PackagesStateRef<'_>> {
        let inner = self.inner.load_full();

        // If the data is new enough, we can use it.
        if let Some(inner) = inner.filter(|x| x.is_new()) {
            return Ok(PackagesStateRef {
                arc: inner,
                _phantom_data: PhantomData,
            });
        }

        self.load_impl(settings, io, http, Some(app_handle), false)
            .await
    }

    pub async fn load_force(
        &self,
        settings: &Settings,
        io: &DefaultEnvironmentIo,
        http: &reqwest::Client,
    ) -> io::Result<PackagesStateRef<'_>> {
        self.load_impl(settings, io, http, None, true).await
    }

    async fn load_impl(
        &self,
        settings: &Settings,
        io: &DefaultEnvironmentIo,
        http: &reqwest::Client,
        app_handle: Option<AppHandle>,
        force: bool,
    ) -> io::Result<PackagesStateRef<'_>> {
        // We won't allow multiple threads to load the data at the same time.
        let guard = self.load_lock.lock().await;

        if !force {
            // if it's not forced, we can check if the data is already loaded.
            let loaded = self.inner.load_full();
            if let Some(loaded) = loaded.filter(|x| x.is_new()) {
                // Another thread loaded it while we were waiting.
                return Ok(PackagesStateRef {
                    arc: loaded,
                    _phantom_data: PhantomData,
                });
            }
        }

        let collection = if let Some(app_handle) = app_handle {
            let collection = PackageCollection::load_cache(settings, io).await?;

            tokio::spawn({
                let mut collection = collection.clone();
                let inner_arc = Arc::clone(&self.inner);
                let io = io.clone();
                let http = http.clone();
                async move {
                    app_handle.emit("package-update-in-progress", true).ok();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    collection.update_cache(&io, &http).await;
                    let arc = Arc::new(PackagesStateInner::new(collect_packages(collection)));
                    inner_arc.store(Some(arc.clone()));
                    app_handle.emit("package-update-in-progress", false).ok();
                    app_handle
                        .emit(
                            "package-update-background",
                            arc.data
                                .get()
                                .packages
                                .iter()
                                .map(crate::commands::TauriPackage::new)
                                .collect::<Vec<_>>(),
                        )
                        .ok();
                }
            });

            collection
        } else {
            PackageCollection::load(settings, io, Some(http)).await?
        };

        fn collect_packages(
            collection: PackageCollection,
        ) -> Yoke<YokeData<'static>, Arc<PackageCollection>> {
            Yoke::<YokeData<'static>, _>::attach_to_cart(Arc::new(collection), |x| {
                YokeData::new(x.get_all_packages().collect())
            })
        }

        let arc = Arc::new(PackagesStateInner::new(collect_packages(collection)));
        self.inner.store(Some(arc.clone()));

        drop(guard);

        Ok(PackagesStateRef {
            arc,
            _phantom_data: PhantomData,
        })
    }

    pub fn get(&self) -> Option<PackagesVersionRef<'_>> {
        let loaded = self.inner.load_full()?;
        Some(PackagesVersionRef {
            arc: loaded,
            _phantom_data: PhantomData,
        })
    }

    pub fn clear_cache(&self) {
        self.inner.store(None);
    }
}

pub struct PackagesStateRef<'a> {
    arc: Arc<PackagesStateInner>,
    _phantom_data: PhantomData<&'a ()>,
}

impl PackagesStateRef<'_> {
    pub fn collection(&self) -> &PackageCollection {
        self.arc.data.backing_cart()
    }

    pub fn packages(&self) -> impl Iterator<Item = &PackageInfo<'_>> {
        self.arc.data.get().packages.iter()
    }

    pub async fn map_yoke<'this, P, F, E, Fut>(
        &'this self,
        f: F,
    ) -> Result<Yoke<P, Arc<PackageCollection>>, E>
    where
        P: for<'a> Yokeable<'a>,
        Fut: Future<Output = Result<<P as Yokeable<'this>>::Output, E>>,
        F: FnOnce(&'this PackageCollection) -> Fut,
    {
        self.arc
            .data
            .try_map_project_async(|collection, _, _| f(collection))
            .await
    }
}

pub struct PackagesVersionRef<'a> {
    arc: Arc<PackagesStateInner>,
    _phantom_data: PhantomData<&'a ()>,
}

#[allow(dead_code)]
impl PackagesVersionRef<'_> {
    pub fn collection(&self) -> &PackageCollection {
        self.arc.data.backing_cart()
    }

    pub fn collection_arc(&self) -> Arc<PackageCollection> {
        self.arc.data.backing_cart().clone()
    }

    pub fn packages(&self) -> &[PackageInfo<'_>] {
        &self.arc.data.get().packages
    }

    pub async fn map_yoke<'this, P, F, E, Fut>(
        &'this self,
        f: F,
    ) -> Result<Yoke<P, Arc<PackageCollection>>, E>
    where
        P: for<'a> Yokeable<'a>,
        Fut: Future<Output = Result<<P as Yokeable<'this>>::Output, E>>,
        F: FnOnce(&'this PackageCollection, &'this [PackageInfo]) -> Fut,
    {
        self.arc
            .data
            .try_map_project_async(|collection, packages, _| f(collection, &packages.packages))
            .await
    }
}

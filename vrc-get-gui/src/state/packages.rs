use arc_swap::ArcSwap;
use std::io;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
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
    pub version: u32,
    pub data: Data,
    pub crated_at: std::time::Instant,
}

impl PackagesStateInner {
    fn new_version() -> u32 {
        static VERSION: AtomicU32 = AtomicU32::new(0);
        VERSION.fetch_add(1, Ordering::AcqRel)
    }

    pub fn uninitialized() -> Self {
        let mut created_at = std::time::Instant::now();
        created_at -= Duration::from_secs(60 * 60);
        Self {
            version: Self::new_version(),
            data: Yoke::attach_to_cart(Arc::new(PackageCollection::empty()), |_| {
                YokeData::new(vec![])
            }),
            crated_at: created_at,
        }
    }

    pub fn new(data: Data) -> Self {
        let version = Self::new_version();
        let crated_at = std::time::Instant::now();
        Self {
            version,
            data,
            crated_at,
        }
    }

    pub fn is_new(&self) -> bool {
        self.crated_at.elapsed() < Duration::from_secs(60 * 5)
    }
}

pub struct PackagesState {
    inner: ArcSwap<PackagesStateInner>,
    load_lock: tokio::sync::Mutex<()>,
}

impl PackagesState {
    pub fn new() -> Self {
        Self {
            inner: ArcSwap::new(Arc::new(PackagesStateInner::uninitialized())),
            load_lock: tokio::sync::Mutex::new(()),
        }
    }

    pub async fn load(
        &self,
        settings: &Settings,
        io: &DefaultEnvironmentIo,
        http: &reqwest::Client,
    ) -> io::Result<PackagesStateRef<'_>> {
        let inner = self.inner.load();

        // If the data is new enough, we can use it.
        if inner.is_new() {
            return Ok(PackagesStateRef {
                arc: inner.clone(),
                _phantom_data: PhantomData,
            });
        }

        self.load_force(settings, io, http).await
    }

    pub async fn load_force(
        &self,
        settings: &Settings,
        io: &DefaultEnvironmentIo,
        http: &reqwest::Client,
    ) -> io::Result<PackagesStateRef> {
        // We won't allow multiple threads to load the data at the same time.
        let guard = self.load_lock.lock().await;

        let loaded = self.inner.load();
        if loaded.is_new() {
            // Another thread loaded it while we were waiting.
            return Ok(PackagesStateRef {
                arc: loaded.clone(),
                _phantom_data: PhantomData,
            });
        }

        let collection = PackageCollection::load(settings, io, Some(http)).await?;

        let yoke = Yoke::<YokeData<'static>, _>::attach_to_cart(Arc::new(collection), |x| {
            YokeData::new(x.get_all_packages().collect())
        });

        let arc = Arc::new(PackagesStateInner::new(yoke));
        self.inner.store(arc.clone());

        drop(guard);

        Ok(PackagesStateRef {
            arc,
            _phantom_data: PhantomData,
        })
    }

    pub fn get_versioned(&self, version: u32) -> Option<PackagesVersionRef<'_>> {
        let loaded = self.inner.load();
        if loaded.version == version {
            Some(PackagesVersionRef {
                arc: loaded.clone(),
                _phantom_data: PhantomData,
            })
        } else {
            None
        }
    }

    pub fn clear_cache(&self) {
        self.inner
            .store(Arc::new(PackagesStateInner::uninitialized()));
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

    pub fn version(&self) -> u32 {
        self.arc.version
    }

    pub fn packages(&self) -> impl Iterator<Item = &PackageInfo> {
        self.arc.data.get().packages.iter()
    }
}

pub struct PackagesVersionRef<'a> {
    arc: Arc<PackagesStateInner>,
    _phantom_data: PhantomData<&'a ()>,
}

impl PackagesVersionRef<'_> {
    pub fn collection(&self) -> &PackageCollection {
        self.arc.data.backing_cart()
    }

    pub fn collection_arc(&self) -> Arc<PackageCollection> {
        self.arc.data.backing_cart().clone()
    }

    pub fn packages(&self) -> &[PackageInfo] {
        &self.arc.data.get().packages
    }
}

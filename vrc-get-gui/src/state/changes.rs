use crate::state::{PackagesStateRef, PackagesVersionRef};
use atomicbox::AtomicOptionBox;
use futures::TryFutureExt;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use vrc_get_vpm::PackageInfo;
use vrc_get_vpm::environment::PackageCollection;
use vrc_get_vpm::unity_project::PendingProjectChanges;
use yoke::{Yoke, Yokeable};

#[derive(Yokeable)]
struct YokeData<'env> {
    changes: PendingProjectChanges<'env>,
}

impl<'env> YokeData<'env> {
    pub fn new(changes: PendingProjectChanges<'env>) -> Self {
        Self { changes }
    }
}

type Data = Yoke<YokeData<'static>, Arc<PackageCollection>>;

struct ChangesStateInner {
    pub version: u32,
    pub data: Data,
}

impl ChangesStateInner {
    fn new_version() -> u32 {
        static VERSION: AtomicU32 = AtomicU32::new(0);
        VERSION.fetch_add(1, Ordering::AcqRel)
    }

    pub fn new(data: Data) -> Self {
        let version = Self::new_version();
        Self { version, data }
    }
}

pub struct ChangesState {
    inner: AtomicOptionBox<ChangesStateInner>,
}

impl ChangesState {
    pub fn new() -> Self {
        Self {
            inner: AtomicOptionBox::none(),
        }
    }

    pub async fn build_changes<'this, F, E, T, Fut>(
        &self,
        packages: &'this PackagesVersionRef<'this>,
        f: F,
        result: impl for<'a> FnOnce(u32, &'a PendingProjectChanges<'a>) -> T,
    ) -> Result<T, E>
    where
        Fut: Future<Output = Result<PendingProjectChanges<'this>, E>>,
        F: FnOnce(&'this PackageCollection, &'this [PackageInfo]) -> Fut,
    {
        let boxed = Box::new(ChangesStateInner::new(
            packages
                .map_yoke(|collection, packages| f(collection, packages).map_ok(YokeData::new))
                .await?,
        ));
        let result = result(boxed.version, &boxed.data.get().changes);
        self.inner.store(Some(boxed), Ordering::SeqCst);
        Ok(result)
    }

    pub async fn build_changes_no_list<'this, F, E, T, Fut>(
        &self,
        packages: &'this PackagesStateRef<'this>,
        f: F,
        result: impl for<'a> FnOnce(u32, &'a PendingProjectChanges<'a>) -> T,
    ) -> Result<T, E>
    where
        Fut: Future<Output = Result<PendingProjectChanges<'this>, E>>,
        F: FnOnce(&'this PackageCollection) -> Fut,
    {
        let boxed = Box::new(ChangesStateInner::new(
            packages
                .map_yoke(|collection| f(collection).map_ok(YokeData::new))
                .await?,
        ));
        let result = result(boxed.version, &boxed.data.get().changes);
        self.inner.store(Some(boxed), Ordering::SeqCst);
        Ok(result)
    }

    pub fn set<T>(
        &self,
        changes: PendingProjectChanges<'static>,
        result: impl for<'a> FnOnce(u32, &'a PendingProjectChanges<'a>) -> T,
    ) -> T {
        let boxed = Box::new(ChangesStateInner::new(Yoke::attach_to_cart(
            Arc::new(PackageCollection::empty()),
            |_| YokeData::new(changes),
        )));
        let result = result(boxed.version, &boxed.data.get().changes);
        self.inner.store(Some(boxed), Ordering::SeqCst);
        result
    }

    pub fn get_versioned(&self, version: u32) -> Option<ChangesVersionRef<'_>> {
        self.inner
            .take(Ordering::SeqCst)
            .filter(|loaded| loaded.version == version)
            .map(|boxed| ChangesVersionRef::new(boxed.data))
    }

    pub fn clear_cache(&self) {
        self.inner.store(None, Ordering::SeqCst);
    }
}

pub struct ChangesVersionRef<'a> {
    _arc: Arc<PackageCollection>,
    data: Option<Data>,
    _phantom_data: PhantomData<&'a ()>,
}

impl ChangesVersionRef<'_> {
    fn new(data: Data) -> Self {
        Self {
            _arc: data.backing_cart().clone(),
            data: Some(data),
            _phantom_data: PhantomData,
        }
    }

    /// Panics if the data has already been taken.
    pub fn take_changes(&mut self) -> PendingProjectChanges {
        let yoke = self.data.take().unwrap();
        // SAFETY: We have clone of backing_cart in self, so yokeable will live while self lives.
        unsafe { yoke.replace_cart(|_| ()) }.into_yokeable().changes
    }
}

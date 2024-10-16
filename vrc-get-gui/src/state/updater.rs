use atomicbox::AtomicOptionBox;
use std::sync::atomic::{AtomicU32, Ordering};
use tauri_plugin_updater::Update;

type Data = Update;

struct UpdateResponseInfo {
    pub version: u32,
    pub data: Data,
}

impl UpdateResponseInfo {
    pub fn new(data: Data) -> Self {
        static VERSION: AtomicU32 = AtomicU32::new(0);

        let version = VERSION.fetch_add(1, Ordering::AcqRel);
        Self { version, data }
    }
}

pub struct UpdaterState {
    inner: AtomicOptionBox<UpdateResponseInfo>,
}

impl UpdaterState {
    pub fn new() -> Self {
        Self {
            inner: AtomicOptionBox::none(),
        }
    }

    pub fn set(&self, data: Data) -> u32 {
        let info = UpdateResponseInfo::new(data);
        let version = info.version;
        self.inner.store(Some(Box::new(info)), Ordering::AcqRel);
        version
    }

    pub fn take(&self) -> Option<UpdaterStateLoadResult> {
        self.inner
            .take(Ordering::AcqRel)
            .map(|inner| UpdaterStateLoadResult { inner: *inner })
    }
}

pub struct UpdaterStateLoadResult {
    inner: UpdateResponseInfo,
}

impl UpdaterStateLoadResult {
    pub fn version(&self) -> u32 {
        self.inner.version
    }

    pub fn into_data(self) -> Data {
        self.inner.data
    }
}

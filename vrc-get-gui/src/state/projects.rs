use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{Mutex, MutexGuard};
use vrc_get_vpm::environment::UserProject;

type Data = Box<[UserProject]>;

struct ProjectsStateInner {
    pub version: u32,
    pub data: Data,
}

impl ProjectsStateInner {
    pub fn new(data: Data) -> Self {
        static VERSION: AtomicU32 = AtomicU32::new(0);

        let version = VERSION.fetch_add(1, Ordering::AcqRel);
        Self { version, data }
    }
}

pub struct ProjectsState {
    inner: Mutex<ProjectsStateInner>,
}

impl ProjectsState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(ProjectsStateInner::new(Default::default())),
        }
    }

    pub async fn set(&self, data: Data) -> ProjectsStateLoadResult<'_> {
        let mut guard = self.inner.lock().await;
        *guard = ProjectsStateInner::new(data);
        ProjectsStateLoadResult { guard }
    }

    pub async fn get(&self) -> ProjectsStateLoadResult {
        ProjectsStateLoadResult {
            guard: self.inner.lock().await,
        }
    }
}

pub struct ProjectsStateLoadResult<'a> {
    guard: MutexGuard<'a, ProjectsStateInner>,
}

#[allow(dead_code)]
impl ProjectsStateLoadResult<'_> {
    pub fn version(&self) -> u32 {
        self.guard.version
    }

    pub fn get(&self, index: usize) -> Option<&UserProject> {
        self.guard.data.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut UserProject> {
        self.guard.data.get_mut(index)
    }

    pub fn data(&self) -> &Data {
        &self.guard.data
    }

    pub fn data_mut(&mut self) -> &mut Data {
        &mut self.guard.data
    }
}

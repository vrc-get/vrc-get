use std::num::Wrapping;
use tauri::{App, Manager};
use tokio::sync::Mutex;
use vrc_get_vpm::io::DefaultEnvironmentIo;
use vrc_get_vpm::unity_project::PendingProjectChanges;

pub fn new_env_state(app: &App) -> impl Send + Sync + 'static {
    Mutex::new(EnvironmentState::new(
        app.state::<DefaultEnvironmentIo>().inner().clone(),
        app.state::<reqwest::Client>().inner().clone(),
    ))
}

unsafe impl Send for EnvironmentState {}

unsafe impl Sync for EnvironmentState {}

pub struct EnvironmentState {
    pub io: DefaultEnvironmentIo,
    pub environment: EnvironmentHolder,
    pub changes_info: crate::commands::ChangesInfoHolder,
}

pub struct PendingProjectChangesInfo<'env> {
    pub environment_version: u32,
    pub changes_version: u32,
    pub changes: PendingProjectChanges<'env>,
}

pub struct EnvironmentHolder {
    pub environment_version: Wrapping<u32>,
}

impl EnvironmentHolder {
    pub fn new() -> Self {
        Self {
            environment_version: Wrapping(0),
        }
    }
}

impl EnvironmentState {
    fn new(io: DefaultEnvironmentIo, _: reqwest::Client) -> Self {
        Self {
            environment: EnvironmentHolder::new(),
            changes_info: crate::commands::ChangesInfoHolder::new(),
            io,
        }
    }
}

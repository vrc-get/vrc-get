use std::io;
use std::num::Wrapping;
use std::ptr::NonNull;

use log::info;
use tokio::sync::Mutex;

use vrc_get_vpm::environment::UserProject;
use vrc_get_vpm::io::DefaultEnvironmentIo;
use vrc_get_vpm::unity_project::PendingProjectChanges;
use vrc_get_vpm::PackageInfo;

use crate::commands::prelude::*;
use crate::commands::project::ChangesInfoHolder;
use crate::commands::util::UpdateResponseHolder;
use crate::config::GuiConfigHolder;

macro_rules! with_environment {
    ($state: expr, |$environment: pat_param$(, $config: pat_param)?| $body: expr) => {{
        let mut state = $state.lock().await;
        let state = &mut *state;
        let $environment = state
            .environment
            .get_environment_mut($crate::commands::state::UpdateRepositoryMode::None, &state.io)
            .await?;
        $(let $config = state.config.load(&state.io).await?;)?
        $body
    }};
}

macro_rules! with_config {
    ($state: expr, |$config: pat_param| $body: expr) => {{
        let mut state = $state.lock().await;
        let state = &mut *state;
        let $config = state.config.load(&state.io).await?;
        $body
    }};
}

pub async fn new_environment(io: &DefaultEnvironmentIo) -> io::Result<Environment> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("vrc-get-litedb/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("building client");
    Environment::load(Some(client), io.clone()).await
}

pub fn new_env_state(io: DefaultEnvironmentIo) -> impl Send + Sync + 'static {
    Mutex::new(EnvironmentState::new(io))
}

unsafe impl Send for EnvironmentState {}

unsafe impl Sync for EnvironmentState {}

pub struct EnvironmentState {
    pub io: DefaultEnvironmentIo,
    pub environment: EnvironmentHolder,
    pub config: GuiConfigHolder,
    pub packages: Option<NonNull<[PackageInfo<'static>]>>,
    // null or reference to
    pub projects: Box<[UserProject]>,
    pub projects_version: Wrapping<u32>,
    pub changes_info: ChangesInfoHolder,
    pub update_response_holder: UpdateResponseHolder,
}

pub struct PendingProjectChangesInfo<'env> {
    pub environment_version: u32,
    pub changes_version: u32,
    pub changes: PendingProjectChanges<'env>,
}

pub struct EnvironmentHolder {
    pub environment: Option<Environment>,
    pub last_update: Option<tokio::time::Instant>,
    pub environment_version: Wrapping<u32>,
    pub last_repository_update: Option<tokio::time::Instant>,
}

impl EnvironmentHolder {
    pub fn new() -> Self {
        Self {
            environment: None,
            last_update: None,
            environment_version: Wrapping(0),
            last_repository_update: None,
        }
    }

    pub async fn get_environment_mut(
        &mut self,
        update_repository: UpdateRepositoryMode,
        io: &DefaultEnvironmentIo,
    ) -> io::Result<&mut Environment> {
        if let Some(ref mut environment) = self.environment {
            if !self
                .last_update
                .map(|x| x.elapsed() < tokio::time::Duration::from_secs(1))
                .unwrap_or(false)
            {
                info!("reloading settings files");
                // reload settings files
                environment.reload().await?;
                self.last_update = Some(tokio::time::Instant::now());
            }

            // outdated after 5 min
            const OUTDATED: tokio::time::Duration = tokio::time::Duration::from_secs(60 * 5);

            match update_repository {
                UpdateRepositoryMode::None => {}
                UpdateRepositoryMode::Force => {
                    self.last_repository_update = Some(tokio::time::Instant::now());
                    self.environment_version += Wrapping(1);
                    info!("loading package infos");
                    environment.load_package_infos(true).await?;
                }
                UpdateRepositoryMode::IfOutdatedOrNecessary => {
                    if self
                        .last_repository_update
                        .map(|x| x.elapsed() > OUTDATED)
                        .unwrap_or(true)
                    {
                        self.last_repository_update = Some(tokio::time::Instant::now());
                        self.environment_version += Wrapping(1);
                        info!("loading package infos");
                        environment.load_package_infos(true).await?;
                    }
                }
                UpdateRepositoryMode::IfOutdatedOrNecessaryForLocal => {
                    if self
                        .last_repository_update
                        .map(|x| x.elapsed() > OUTDATED)
                        .unwrap_or(true)
                    {
                        self.last_repository_update = Some(tokio::time::Instant::now());
                        self.environment_version += Wrapping(1);
                        info!("loading local package infos");
                        environment.load_user_package_infos().await?;
                    }
                }
            }

            Ok(environment)
        } else {
            self.environment = Some(new_environment(io).await?);
            self.last_update = Some(tokio::time::Instant::now());
            let environment = self.environment.as_mut().unwrap();

            match update_repository {
                UpdateRepositoryMode::None => {}
                UpdateRepositoryMode::Force | UpdateRepositoryMode::IfOutdatedOrNecessary => {
                    self.last_repository_update = Some(tokio::time::Instant::now());
                    self.environment_version += Wrapping(1);
                    info!("loading package infos");
                    environment.load_package_infos(true).await?;
                }
                UpdateRepositoryMode::IfOutdatedOrNecessaryForLocal => {
                    self.last_repository_update = Some(tokio::time::Instant::now());
                    self.environment_version += Wrapping(1);
                    info!("loading local package infos");
                    environment.load_user_package_infos().await?;
                }
            }

            Ok(environment)
        }
    }
}

pub enum UpdateRepositoryMode {
    None,
    Force,
    IfOutdatedOrNecessary,
    IfOutdatedOrNecessaryForLocal,
}

impl EnvironmentState {
    fn new(io: DefaultEnvironmentIo) -> Self {
        Self {
            environment: EnvironmentHolder::new(),
            config: GuiConfigHolder::new(),
            packages: None,
            projects: Box::new([]),
            projects_version: Wrapping(0),
            changes_info: ChangesInfoHolder::new(),
            update_response_holder: UpdateResponseHolder::new(),
            io,
        }
    }
}

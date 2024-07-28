use crate::commands::prelude::*;
use crate::commands::project::ChangesInfoHolder;
use crate::commands::util::UpdateResponseHolder;
use log::info;
use std::backtrace::Backtrace;
use std::io;
use std::mem::forget;
use std::num::Wrapping;
use std::ops::{Deref, DerefMut};
use std::time::{Duration, Instant};
use tauri::{App, Manager};
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};
use vrc_get_vpm::environment::{PackageCollection, Settings, UserProject};
use vrc_get_vpm::io::DefaultEnvironmentIo;
use vrc_get_vpm::unity_project::PendingProjectChanges;
use vrc_get_vpm::PackageInfo;
use yoke::{Yoke, Yokeable};

macro_rules! with_environment {
    ($state: expr, |$environment: pat_param| $body: expr) => {{
        let mut state = $state.lock().await;
        let state = &mut *state;
        let $environment = state
            .environment
            .get_environment_mut(
                $crate::commands::state::UpdateRepositoryMode::None,
                &state.io,
                &state.http,
            )
            .await?;
        $body
    }};
}

pub fn new_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("vrc-get-litedb/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("building client")
}

pub async fn new_environment(io: &DefaultEnvironmentIo) -> io::Result<Environment> {
    Environment::load(io).await
}

pub fn new_env_state(app: &App) -> impl Send + Sync + 'static {
    Mutex::new(EnvironmentState::new(
        app.state::<DefaultEnvironmentIo>().inner().clone(),
        app.state::<reqwest::Client>().inner().clone(),
    ))
}

unsafe impl Send for EnvironmentState {}

unsafe impl Sync for EnvironmentState {}

#[derive(Yokeable)]
pub struct PackageList<'env> {
    pub packages: Vec<PackageInfo<'env>>,
}

impl<'a> FromIterator<PackageInfo<'a>> for PackageList<'a> {
    fn from_iter<T: IntoIterator<Item = PackageInfo<'a>>>(iter: T) -> Self {
        Self {
            packages: iter.into_iter().collect(),
        }
    }
}

pub struct EnvironmentState {
    pub io: DefaultEnvironmentIo,
    pub http: reqwest::Client,
    pub environment: EnvironmentHolder,
    pub packages: Option<Yoke<PackageList<'static>, Box<PackageCollection>>>,
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
        http: &reqwest::Client,
    ) -> io::Result<&mut Environment> {
        if let Some(ref mut environment) = self.environment {
            if !self
                .last_update
                .map(|x| x.elapsed() < tokio::time::Duration::from_secs(1))
                .unwrap_or(false)
            {
                info!("reloading settings files");
                // reload settings files
                environment.reload(io).await?;
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
                    environment.load_package_infos(io, Some(http)).await?;
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
                        environment.load_package_infos(io, Some(http)).await?;
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
                        environment.load_user_package_infos(io).await?;
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
                    environment.load_package_infos(io, Some(http)).await?;
                }
                UpdateRepositoryMode::IfOutdatedOrNecessaryForLocal => {
                    self.last_repository_update = Some(tokio::time::Instant::now());
                    self.environment_version += Wrapping(1);
                    info!("loading local package infos");
                    environment.load_user_package_infos(io).await?;
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
    fn new(io: DefaultEnvironmentIo, http: reqwest::Client) -> Self {
        Self {
            environment: EnvironmentHolder::new(),
            packages: None,
            projects: Box::new([]),
            projects_version: Wrapping(0),
            changes_info: ChangesInfoHolder::new(),
            update_response_holder: UpdateResponseHolder::new(),
            io,
            http,
        }
    }
}

struct SettingsInner {
    settings: Settings,
    loaded_at: Instant,
}

impl SettingsInner {
    fn is_new(&self) -> bool {
        self.loaded_at + Duration::from_secs(1) < Instant::now()
    }
}

// TODO: This is a temporary implementation. We may avoid lock on read-only access
pub struct SettingsState {
    // None: not loaded yet, Some: loaded but might be outdated
    mut_lock: Mutex<Option<SettingsInner>>,
}

impl SettingsState {
    pub async fn new() -> io::Result<Self> {
        Ok(Self {
            mut_lock: Mutex::new(None),
        })
    }

    pub async fn load(&self, io: &DefaultEnvironmentIo) -> io::Result<SettingsRef> {
        Ok(SettingsRef::new(self.do_load(io).await?))
    }

    async fn do_load<'a>(
        &'a self,
        io: &DefaultEnvironmentIo,
    ) -> io::Result<MappedMutexGuard<'a, SettingsInner>> {
        let mut lock = self.mut_lock.lock().await;

        match *lock {
            Some(ref inner) if inner.is_new() => {}
            _ => {
                *lock = Some(SettingsInner {
                    settings: Settings::load(io).await?,
                    loaded_at: Instant::now(),
                })
            }
        }

        Ok(MutexGuard::map(lock, |x| x.as_mut().unwrap()))
    }

    pub async fn load_mut<'a>(
        &'a self,
        io: &'a DefaultEnvironmentIo,
    ) -> io::Result<SettingMutRef<'a>> {
        Ok(SettingMutRef {
            lock: self.do_load(io).await?,
            io,
            save_checker: UnsavedDropChecker::new(),
        })
    }
}

pub struct SettingsRef<'a> {
    state: MappedMutexGuard<'a, SettingsInner>,
}

impl<'a> SettingsRef<'a> {
    fn new(state: MappedMutexGuard<'a, SettingsInner>) -> Self {
        Self { state }
    }
}

impl Deref for SettingsRef<'_> {
    type Target = Settings;

    #[inline(always)]
    fn deref(&self) -> &Settings {
        &self.state.settings
    }
}

pub struct SettingMutRef<'s> {
    lock: MappedMutexGuard<'s, SettingsInner>,
    io: &'s DefaultEnvironmentIo,
    save_checker: UnsavedDropChecker,
}

impl SettingMutRef<'_> {
    pub async fn save(self) -> io::Result<()> {
        forget(self.save_checker); // We're doing the save, so we don't need to check for it
        self.lock.settings.save(self.io).await?;
        Ok(())
    }

    pub fn require_save(&mut self) {
        self.save_checker.require_save();
    }

    pub async fn maybe_save(self) -> io::Result<()> {
        if self.save_checker.should_save {
            self.save().await
        } else {
            // skip should_save in drop
            forget(self);
            Ok(())
        }
    }
}

impl Deref for SettingMutRef<'_> {
    type Target = Settings;

    #[inline(always)]
    fn deref(&self) -> &Settings {
        &self.lock.settings
    }
}

impl DerefMut for SettingMutRef<'_> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Settings {
        &mut self.lock.settings
    }
}

struct UnsavedDropChecker {
    should_save: bool,
}

impl UnsavedDropChecker {
    fn new() -> Self {
        Self { should_save: false }
    }

    pub(crate) fn require_save(&mut self) {
        self.should_save = true;
    }
}

impl Drop for UnsavedDropChecker {
    fn drop(&mut self) {
        if self.should_save {
            let trace = Backtrace::capture();
            log::error!("dropped without save: {trace}");
        }
    }
}

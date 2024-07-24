use std::future::{ready, Future};
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::sync::Arc;

use arc_swap::ArcSwapOption;
use futures::*;
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, MutexGuard};

use vrc_get_vpm::io::{DefaultEnvironmentIo, EnvironmentIo};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiConfig {
    #[serde(default)]
    pub gui_hidden_repositories: IndexSet<String>,
    #[serde(default)]
    pub hide_local_user_packages: bool,
    #[serde(default)]
    pub window_size: WindowSize,
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default = "language_default")]
    pub language: String,
    #[serde(default = "theme_default")]
    pub theme: String,
    #[serde(default = "backup_default")]
    pub backup_format: String,
    #[serde(default = "project_sorting_default")]
    pub project_sorting: String,
    #[serde(default = "release_channel_default")]
    // "stable" or "beta"
    pub release_channel: String,
    #[serde(default)]
    pub use_alcom_for_vcc_protocol: bool,
    #[serde(default)]
    pub setup_process_progress: u32,
    #[serde(default)]
    pub default_unity_arguments: Option<Vec<String>>,
}

impl Default for GuiConfig {
    fn default() -> Self {
        GuiConfig {
            gui_hidden_repositories: IndexSet::new(),
            hide_local_user_packages: false,
            window_size: WindowSize::default(),
            fullscreen: false,
            language: language_default(),
            theme: theme_default(),
            backup_format: backup_default(),
            project_sorting: project_sorting_default(),
            release_channel: release_channel_default(),
            use_alcom_for_vcc_protocol: false,
            setup_process_progress: 0,
            default_unity_arguments: None,
        }
    }
}

impl GuiConfig {
    pub(crate) fn fix_defaults(&mut self) {
        if self.language.is_empty() {
            self.language = language_default();
        }
        if self.language == "zh_cn" {
            self.language = "zh_hans".to_string();
        }
        if self.backup_format.is_empty() {
            self.backup_format = backup_default();
        }
        if self.project_sorting.is_empty() {
            self.project_sorting = project_sorting_default();
        }
    }
}

fn language_default() -> String {
    for locale in sys_locale::get_locales() {
        if locale.starts_with("en") {
            return "en".to_string();
        }
        if locale.starts_with("de") {
            return "de".to_string();
        }
        if locale.starts_with("ja") {
            return "ja".to_string();
        }
        if locale.starts_with("zh") {
            return "zh_hans".to_string();
        }
    }

    "en".to_string()
}

fn theme_default() -> String {
    "system".to_string()
}

fn backup_default() -> String {
    "default".to_string()
}

fn project_sorting_default() -> String {
    "lastModified".to_string()
}

fn release_channel_default() -> String {
    "stable".to_string()
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl Default for WindowSize {
    fn default() -> Self {
        WindowSize {
            width: 1300,
            height: 800,
        }
    }
}

struct GuiConfigStateInner {
    config: GuiConfig,
    path: PathBuf,
}

pub struct GuiConfigState {
    inner: ArcSwapOption<GuiConfigStateInner>,
    mut_lock: Mutex<()>,
}

impl GuiConfigState {
    pub fn new() -> Self {
        Self {
            inner: ArcSwapOption::new(None),
            mut_lock: Mutex::new(()),
        }
    }

    pub async fn load(&self, io: &DefaultEnvironmentIo) -> io::Result<GuiConfigRef> {
        Self::load_async_impl(&self.inner, io).await
    }

    async fn load_async_impl(
        inner: &ArcSwapOption<GuiConfigStateInner>,
        io: &DefaultEnvironmentIo,
    ) -> io::Result<GuiConfigRef> {
        if let Some(inner) = &*inner.load() {
            Ok(GuiConfigRef::new(inner.clone()))
        } else {
            Ok(GuiConfigRef::new(Self::set_updated_or_removed(
                inner,
                load_async(io).await?,
            )))
        }
    }

    #[allow(dead_code)] // Not used in the current codebase but used soon
    pub fn load_sync(&self, io: &DefaultEnvironmentIo) -> io::Result<GuiConfigRef> {
        let inner = self.inner.load();
        if let Some(inner) = &*inner {
            Ok(GuiConfigRef::new(inner.clone()))
        } else {
            Ok(GuiConfigRef::new(Self::set_updated_or_removed(
                &self.inner,
                load_sync(io)?,
            )))
        }
    }

    fn set_updated_or_removed(
        inner: &ArcSwapOption<GuiConfigStateInner>,
        value: GuiConfigStateInner,
    ) -> Arc<GuiConfigStateInner> {
        let arc = Arc::new(value);
        let guard = inner.compare_and_swap(std::ptr::null(), Some(arc.clone()));
        if let Some(old) = &*guard {
            old.clone()
        } else {
            arc
        }
    }

    pub async fn load_mut<'s>(
        &'s self,
        io: &DefaultEnvironmentIo,
    ) -> io::Result<GuiConfigMutRef<'s>> {
        let lock = self.mut_lock.lock().await;
        let loaded = Self::load_async_impl(&self.inner, io).await?;
        Ok(GuiConfigMutRef {
            config: loaded.state.config.clone(),
            path: loaded.state.path.clone(),
            _mut_lock_guard: lock,
            cache: &self.inner,
        })
    }
}

pub struct GuiConfigRef {
    state: Arc<GuiConfigStateInner>,
}

impl GuiConfigRef {
    fn new(state: Arc<GuiConfigStateInner>) -> Self {
        Self { state }
    }
}

impl Deref for GuiConfigRef {
    type Target = GuiConfig;

    #[inline(always)]
    fn deref(&self) -> &GuiConfig {
        &self.state.config
    }
}

pub struct GuiConfigMutRef<'s> {
    config: GuiConfig,
    path: PathBuf,
    _mut_lock_guard: MutexGuard<'s, ()>,
    cache: &'s ArcSwapOption<GuiConfigStateInner>,
}

impl GuiConfigMutRef<'_> {
    pub async fn save(self) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.config)?;
        tokio::fs::create_dir_all(self.path.parent().unwrap()).await?;
        let mut file = tokio::fs::File::create(&self.path).await?;
        file.write_all(json.as_bytes()).await?;
        file.sync_data().await?;
        drop(file);
        self.cache.swap(Some(Arc::new(GuiConfigStateInner {
            config: self.config,
            path: self.path,
        })));
        Ok(())
    }
}

impl Deref for GuiConfigMutRef<'_> {
    type Target = GuiConfig;

    #[inline(always)]
    fn deref(&self) -> &GuiConfig {
        &self.config
    }
}

impl DerefMut for GuiConfigMutRef<'_> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut GuiConfig {
        &mut self.config
    }
}

trait FsWrapper {
    fn read(path: &Path) -> impl Future<Output = io::Result<Vec<u8>>> + Send;
    fn rename(from: &Path, to: &Path) -> impl Future<Output = io::Result<()>> + Send;
}

async fn loader<F: FsWrapper>(path: PathBuf) -> io::Result<GuiConfigStateInner> {
    async fn load_fs<F: FsWrapper>(path: &PathBuf) -> io::Result<GuiConfig> {
        match F::read(&path).await {
            Ok(buffer) => {
                let mut loaded = serde_json::from_slice::<GuiConfig>(&buffer)?;
                loaded.fix_defaults();
                Ok(loaded)
            }
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(GuiConfig::default()),
            Err(e) => return Err(e),
        }
    }

    async fn backup_old_config<F: FsWrapper>(path: &PathBuf) -> io::Result<()> {
        let mut i = 0;
        loop {
            let backup_path = path.with_extension(format!("json.bak.{}", i));
            match F::rename(path, &backup_path).await {
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    i = i + 1;
                }
                Ok(()) => break Ok(()),
                Err(e) if e.kind() == io::ErrorKind::NotFound => break Ok(()),
                Err(e) => break Err(e),
            }
        }
    }

    let config = match load_fs::<F>(&path).await {
        Ok(loaded) => loaded,
        Err(e) => {
            log::error!(
                "Failed to load gui-config.json, using default config: {}",
                e
            );

            // backup old config if possible
            if let Err(e) = backup_old_config::<F>(&path).await {
                log::error!("Failed to backup old config: {}", e);
            }

            GuiConfig::default()
        }
    };

    Ok(GuiConfigStateInner { config, path })
}

async fn load_async(io: &DefaultEnvironmentIo) -> io::Result<GuiConfigStateInner> {
    struct AsyncIO;
    impl FsWrapper for AsyncIO {
        fn read(path: &Path) -> impl Future<Output = io::Result<Vec<u8>>> + Send {
            tokio::fs::read(path)
        }

        fn rename(from: &Path, to: &Path) -> impl Future<Output = io::Result<()>> + Send {
            tokio::fs::rename(from, to)
        }
    }

    let path = io.resolve("vrc-get/gui-config.json".as_ref());

    loader::<AsyncIO>(path).await
}

fn load_sync(io: &DefaultEnvironmentIo) -> io::Result<GuiConfigStateInner> {
    struct SyncIO;
    impl FsWrapper for SyncIO {
        fn read(path: &Path) -> impl Future<Output = io::Result<Vec<u8>>> + Send {
            ready(std::fs::read(path))
        }

        fn rename(from: &Path, to: &Path) -> impl Future<Output = io::Result<()>> + Send {
            ready(std::fs::rename(from, to))
        }
    }

    let path = io.resolve("vrc-get/gui-config.json".as_ref());

    loader::<SyncIO>(path)
        .now_or_never()
        .expect("sync operation suspended")
}

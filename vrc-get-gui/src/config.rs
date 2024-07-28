use std::future::Future;
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwap;
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
    inner: ArcSwap<GuiConfigStateInner>,
    mut_lock: Mutex<()>,
}

impl GuiConfigState {
    pub async fn new_load(io: &DefaultEnvironmentIo) -> io::Result<Self> {
        let loaded = load_async(io).await?;
        Ok(Self {
            inner: ArcSwap::new(Arc::new(loaded)),
            mut_lock: Mutex::new(()),
        })
    }

    pub fn get(&self) -> GuiConfigRef {
        GuiConfigRef::new(self.inner.load().clone())
    }

    pub async fn load_mut(&self) -> io::Result<GuiConfigMutRef<'_>> {
        let lock = self.mut_lock.lock().await;
        let loaded = GuiConfigRef::new(self.inner.load().clone());
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
    cache: &'s ArcSwap<GuiConfigStateInner>,
}

impl GuiConfigMutRef<'_> {
    pub async fn save(self) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.config)?;
        tokio::fs::create_dir_all(self.path.parent().unwrap()).await?;
        let mut file = tokio::fs::File::create(&self.path).await?;
        file.write_all(json.as_bytes()).await?;
        file.sync_data().await?;
        drop(file);
        self.cache.swap(Arc::new(GuiConfigStateInner {
            config: self.config,
            path: self.path,
        }));
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
    async fn load_fs<F: FsWrapper>(path: &Path) -> io::Result<GuiConfig> {
        match F::read(path).await {
            Ok(buffer) => {
                let mut loaded = serde_json::from_slice::<GuiConfig>(&buffer)?;
                loaded.fix_defaults();
                Ok(loaded)
            }
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(GuiConfig::default()),
            Err(e) => Err(e),
        }
    }

    async fn backup_old_config<F: FsWrapper>(path: &Path) -> io::Result<()> {
        let mut i = 0;
        loop {
            let backup_path = path.with_extension(format!("json.bak.{}", i));
            match F::rename(path, &backup_path).await {
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    i += 1;
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

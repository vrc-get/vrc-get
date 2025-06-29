use crate::config::GuiConfig;
use arc_swap::ArcSwap;
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, MutexGuard};
use vrc_get_vpm::io::DefaultEnvironmentIo;

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
        GuiConfigRef::new(self.inner.load_full())
    }

    pub async fn load_mut(&self) -> io::Result<GuiConfigMutRef<'_>> {
        let lock = self.mut_lock.lock().await;
        let loaded = GuiConfigRef::new(self.inner.load_full());
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

async fn load_async(io: &DefaultEnvironmentIo) -> io::Result<GuiConfigStateInner> {
    async fn load_fs(path: &Path) -> io::Result<GuiConfig> {
        match tokio::fs::read(path).await {
            Ok(buffer) => {
                let mut loaded = serde_json::from_slice::<GuiConfig>(&buffer)?;
                loaded.fix_defaults();
                Ok(loaded)
            }
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(GuiConfig::default()),
            Err(e) => Err(e),
        }
    }

    async fn backup_old_config(path: &Path) -> io::Result<()> {
        let mut i = 0;
        loop {
            let backup_path = path.with_extension(format!("json.bak.{i}"));
            match tokio::fs::rename(path, &backup_path).await {
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    i += 1;
                }
                Ok(()) => break Ok(()),
                Err(e) if e.kind() == io::ErrorKind::NotFound => break Ok(()),
                Err(e) => break Err(e),
            }
        }
    }

    let path = io.resolve("vrc-get/gui-config.json".as_ref());

    let config = match load_fs(&path).await {
        Ok(loaded) => loaded,
        Err(e) => {
            log::error!("Failed to load gui-config.json, using default config: {e}");

            // backup old config if possible
            if let Err(e) = backup_old_config(&path).await {
                log::error!("Failed to backup old config: {e}");
            }

            GuiConfig::default()
        }
    };

    Ok(GuiConfigStateInner { config, path })
}

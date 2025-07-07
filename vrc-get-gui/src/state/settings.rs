use arc_swap::ArcSwapOption;
use std::backtrace::Backtrace;
use std::io;
use std::marker::PhantomData;
use std::mem::forget;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::thread::panicking;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, MutexGuard};
use vrc_get_vpm::environment::Settings;
use vrc_get_vpm::io::DefaultEnvironmentIo;

#[derive(Clone)]
struct SettingsInner {
    settings: Settings,
    loaded_at: Instant,
}

impl SettingsInner {
    fn new(settings: Settings) -> Self {
        Self {
            settings,
            loaded_at: Instant::now(),
        }
    }

    fn is_new(&self) -> bool {
        self.loaded_at.elapsed() < Duration::from_secs(1)
    }
}

pub struct SettingsState {
    inner: ArcSwapOption<SettingsInner>,
    load_lock: Mutex<()>,
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            inner: ArcSwapOption::default(),
            load_lock: Mutex::new(()),
        }
    }

    pub async fn load(&self, io: &DefaultEnvironmentIo) -> io::Result<SettingsRef<'_>> {
        // If the data is new enough, we can use it.
        let inner = self.inner.load_full();
        if let Some(inner) = inner.filter(|x| x.is_new()) {
            return Ok(SettingsRef::new(inner));
        }

        let guard = self.load_lock.lock().await;

        // Recheck after lock to get loaded from another thread
        let inner = self.inner.load_full();
        if let Some(inner) = inner.filter(|x| x.is_new()) {
            return Ok(SettingsRef::new(inner));
        }

        // Loaded data is too old, reload it
        let arc = Arc::new(SettingsInner::new(Settings::load(io).await?));

        self.inner.store(Some(arc.clone()));

        drop(guard); // free the lock

        Ok(SettingsRef::new(arc))
    }

    pub async fn load_mut<'a>(
        &'a self,
        io: &'a DefaultEnvironmentIo,
    ) -> io::Result<SettingMutRef<'a>> {
        // since we're editing, do everything in the lock
        let guard = self.load_lock.lock().await;

        // if loaded one is new enough, we can use it
        let inner = self.inner.load_full();
        if let Some(inner) = inner.filter(|x| x.is_new()) {
            self.inner.store(None); // remove the old one

            let settings = Arc::try_unwrap(inner).unwrap_or_else(|arc| {
                log::info!("Unwrapping settings arc failed, cloning...");
                SettingsInner::clone(&arc)
            });

            return Ok(SettingMutRef::new(settings, &self.inner, io, guard));
        }

        // otherwise, if loaded one is old, load and use it

        let loaded = SettingsInner::new(Settings::load(io).await?);

        Ok(SettingMutRef::new(loaded, &self.inner, io, guard))
    }
}

pub struct SettingsRef<'a> {
    arc: Arc<SettingsInner>,
    _phantom_data: PhantomData<&'a ()>,
}

impl SettingsRef<'_> {
    fn new(arc: Arc<SettingsInner>) -> Self {
        Self {
            arc,
            _phantom_data: PhantomData,
        }
    }
}

impl Deref for SettingsRef<'_> {
    type Target = Settings;

    #[inline(always)]
    fn deref(&self) -> &Settings {
        &self.arc.settings
    }
}

pub struct SettingMutRef<'s> {
    owned: SettingsInner,
    cache_slot: &'s ArcSwapOption<SettingsInner>,
    guard: MutexGuard<'s, ()>,
    io: &'s DefaultEnvironmentIo,
    save_checker: UnsavedDropChecker,
}

impl<'s> SettingMutRef<'s> {
    fn new(
        settings: SettingsInner,
        inner: &'s ArcSwapOption<SettingsInner>,
        io: &'s DefaultEnvironmentIo,
        guard: MutexGuard<'s, ()>,
    ) -> Self {
        Self {
            owned: settings,
            cache_slot: inner,
            guard,
            io,
            save_checker: UnsavedDropChecker::new(),
        }
    }

    pub async fn save(self) -> io::Result<()> {
        // We're doing the save, so we don't need to check for it
        forget(self.save_checker);
        // first, save the settings
        self.owned.settings.save(self.io).await?;
        // then, save to cache
        // since we've saved that, renew the loaded_at
        self.cache_slot
            .store(Some(Arc::new(SettingsInner::new(self.owned.settings))));
        // finally, release the lock
        drop(self.guard);
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
            forget(self.save_checker);
            // we're owning the settings so return to cache
            self.cache_slot.store(Some(Arc::new(self.owned)));
            Ok(())
        }
    }
}

impl Deref for SettingMutRef<'_> {
    type Target = Settings;

    #[inline(always)]
    fn deref(&self) -> &Settings {
        &self.owned.settings
    }
}

impl DerefMut for SettingMutRef<'_> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Settings {
        &mut self.owned.settings
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
        if self.should_save && !panicking() {
            let trace = Backtrace::capture();
            log::error!("dropped without save: {trace}");
        }
    }
}

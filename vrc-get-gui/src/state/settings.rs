use std::backtrace::Backtrace;
use std::io;
use std::mem::forget;
use std::ops::{Deref, DerefMut};
use std::time::{Duration, Instant};
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};
use vrc_get_vpm::environment::Settings;
use vrc_get_vpm::io::DefaultEnvironmentIo;

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
    pub fn new() -> Self {
        Self {
            mut_lock: Mutex::new(None),
        }
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

use crate::io;
use crate::io::IoTrait;
use crate::utils::to_vec_pretty_os_eol;
use serde::Serialize;
use std::ops::Deref;
use std::path::Path;

#[derive(Debug)]
pub(crate) struct SaveController<T> {
    parsed: T,
    settings_changed: bool,
}

impl<T> SaveController<T> {
    pub(crate) fn new(parsed: T) -> Self {
        Self {
            parsed,
            settings_changed: false,
        }
    }

    #[inline]
    pub(crate) fn may_changing(&mut self, f: impl FnOnce(&mut T) -> bool) {
        if f(&mut self.parsed) {
            self.settings_changed = true;
        }
    }

    #[inline]
    pub(crate) fn as_mut(&mut self) -> &mut T {
        self.settings_changed = true;
        &mut self.parsed
    }
}

impl<T: Serialize> SaveController<T> {
    pub(crate) async fn save(&mut self, io: &impl IoTrait, path: &Path) -> io::Result<()> {
        if !self.settings_changed {
            return Ok(());
        }

        io.create_dir_all(path.parent().unwrap_or("".as_ref()))
            .await?;
        io.write(path, &to_vec_pretty_os_eol(&self.parsed)?).await?;

        self.settings_changed = false;
        Ok(())
    }
}

impl<T> Deref for SaveController<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.parsed
    }
}

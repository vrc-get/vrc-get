#![allow(unsafe_code)]

use crate::io;
use crate::io::EnvironmentIo;
use vrc_get_litedb::DatabaseConnection;

pub struct VccDatabaseConnection {
    pub(crate) db: DatabaseConnection,
    _guard: has_drop::MutexGuard,
}

impl VccDatabaseConnection {
    pub async fn connect(io: &impl EnvironmentIo) -> io::Result<Self> {
        io.connect_lite_db().await
    }

    pub(crate) fn new(db: DatabaseConnection, _guard: has_drop::MutexGuard) -> Self {
        Self { db, _guard }
    }

    pub async fn save(&self, _: &impl EnvironmentIo) -> io::Result<()> {
        // nop for now but might have to do something in the future
        Ok(())
    }
}

mod has_drop {
    pub type MutexGuard = Box<dyn HasDrop>;
    pub trait HasDrop: Send + Sync {}
    impl<T: ?Sized + Send + Sync> HasDrop for T {}
}

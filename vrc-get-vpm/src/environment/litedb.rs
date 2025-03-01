#![allow(unsafe_code)]

use crate::io;
use crate::io::EnvironmentIo;
use vrc_get_litedb::engine::LiteEngine;
use vrc_get_litedb::expression::BsonExpression;

pub struct VccDatabaseConnection {
    pub(crate) db: LiteEngine,
    _guard: has_drop::MutexGuard,
}

impl VccDatabaseConnection {
    pub async fn connect(io: &impl EnvironmentIo) -> io::Result<Self> {
        let database = io.connect_lite_db().await?;

        database
            .db
            .ensure_index(
                "projects",
                "Path",
                BsonExpression::create("$.Path").unwrap(),
                false, // why? but upstream does so
            )
            .await?;

        Ok(database)
    }

    pub(crate) fn new(db: LiteEngine, _guard: has_drop::MutexGuard) -> Self {
        Self { db, _guard }
    }

    pub async fn save(&self, _: &impl EnvironmentIo) -> io::Result<()> {
        // nop for now but might have to do something in the future
        self.db.checkpoint().await?;
        Ok(())
    }
}

mod has_drop {
    pub type MutexGuard = Box<dyn HasDrop>;
    pub trait HasDrop: Send + Sync {}
    impl<T: ?Sized + Send + Sync> HasDrop for T {}
}

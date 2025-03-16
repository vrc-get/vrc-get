#![allow(unsafe_code)]

use crate::io;
use crate::io::EnvironmentIo;
use futures::prelude::*;
use std::pin::pin;
use vrc_get_litedb::expression::BsonExpression;
use vrc_get_litedb::file_io::LiteDBFile;

pub struct VccDatabaseConnection {
    pub(crate) db: LiteDBFile,
    _guard: has_drop::MutexGuard,
}

static FILE_NAME: &str = "vcc.liteDB";

impl VccDatabaseConnection {
    pub async fn connect(io: &impl EnvironmentIo) -> io::Result<Self> {
        let mut buffer = vec![];

        let path = io.resolve(FILE_NAME.as_ref());

        let lock = {
            use sha1::Digest;

            let path = path.to_string_lossy();
            let path_lower = path.to_lowercase();
            let mut sha1 = sha1::Sha1::new();
            sha1.update(path_lower.as_bytes());
            let hash = &sha1.finalize()[..];
            let hash_hex = hex::encode(hash);
            // this lock name is same as shared engine in litedb
            let name = format!("Global\\{hash_hex}.Mutex");

            Box::new(io.new_mutex(name.as_ref()).await?)
        };

        pin!(io.open(FILE_NAME.as_ref()).await?)
            .read_to_end(&mut buffer)
            .await?;

        let mut litedb = LiteDBFile::parse(&buffer)?;

        litedb
            .ensure_index(
                "projects",
                "Path",
                BsonExpression::create("$.Path").unwrap(),
                false, // why? but upstream does so
            )
            .expect("index is do");

        Ok(Self::new(litedb, lock))
    }

    pub(crate) fn new(db: LiteDBFile, _guard: has_drop::MutexGuard) -> Self {
        Self { db, _guard }
    }

    pub async fn save(&self, io: &impl EnvironmentIo) -> io::Result<()> {
        // nop for now but might have to do something in the future
        io.write_sync(FILE_NAME.as_ref(), &self.db.serialize())
            .await?;
        Ok(())
    }
}

mod has_drop {
    pub type MutexGuard = Box<dyn HasDrop>;
    pub trait HasDrop: Send + Sync {}
    impl<T: ?Sized + Send + Sync> HasDrop for T {}
}

#![allow(unsafe_code)]

use crate::io;
use crate::io::EnvironmentIo;
use vrc_get_litedb::DatabaseConnection;

pub struct VccDatabaseConnection {
    pub(crate) db: DatabaseConnection,
}

impl VccDatabaseConnection {
    pub fn connect(io: &impl EnvironmentIo) -> io::Result<Self> {
        Ok(Self {
            db: io.connect_lite_db()?,
        })
    }

    pub async fn save(&self, _: &impl EnvironmentIo) -> io::Result<()> {
        // nop for now but might have to do something in the future
        Ok(())
    }
}

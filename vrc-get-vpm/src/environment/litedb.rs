#![allow(unsafe_code)]

use crate::io::EnvironmentIo;
use crate::{io, Environment, HttpClient};
use vrc_get_litedb::DatabaseConnection;

pub struct VccDatabaseConnection {
    pub(crate) connection: DatabaseConnection,
}

impl VccDatabaseConnection {
    pub fn connect(io: &impl EnvironmentIo) -> io::Result<Self> {
        Ok(Self {
            connection: io.connect_lite_db()?,
        })
    }

    pub async fn save(&self, _: &impl EnvironmentIo) -> io::Result<()> {
        // nop for now but might have to do something in the future
        Ok(())
    }

    pub(crate) fn db(&self) -> &DatabaseConnection {
        &self.connection
    }
}

impl<T: HttpClient> Environment<T> {
    pub fn load_from_db(&mut self, connection: &VccDatabaseConnection) -> io::Result<()> {
        self.settings.load_from_db(connection)
    }
}

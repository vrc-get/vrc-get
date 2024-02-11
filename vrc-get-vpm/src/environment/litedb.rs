use crate::io::EnvironmentIo;
use crate::{io, Environment, HttpClient};
use vrc_get_litedb::DatabaseConnection;

impl<T: HttpClient, IO: EnvironmentIo> Environment<T, IO> {
    // TODO?: use inner mutability to get the database connection?
    pub(super) fn get_db(&mut self) -> io::Result<&DatabaseConnection> {
        if self.litedb_connection.is_none() {
            self.litedb_connection = Some(self.io.connect_lite_db()?);
        }

        Ok(self.litedb_connection.as_ref().unwrap())
    }
}

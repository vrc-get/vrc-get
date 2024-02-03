use crate::connection_string::{ConnectionString, ConnectionStringFFI};
use crate::lowlevel;
use super::Result;

pub struct DatabaseConnection {
    ptr: lowlevel::GcHandle,
}

impl DatabaseConnection {
    pub(crate) fn connect(string: &ConnectionString) -> Result<DatabaseConnection> {
        unsafe {
            vrc_get_litedb_database_connection_new(&ConnectionStringFFI::from(string))
                .into_result()
                .map(|ptr| DatabaseConnection { ptr })
        }
    }
}

impl Drop for DatabaseConnection {
    fn drop(&mut self) {
        unsafe {
            vrc_get_litedb_database_connection_dispose(self.ptr.get());
        }
    }
}

// C# functions
extern "C" {
    fn vrc_get_litedb_database_connection_new(string: &ConnectionStringFFI) -> super::error::HandleErrorResult;
    fn vrc_get_litedb_database_connection_dispose(ptr: isize);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connect() {
        ConnectionString::new("vcc.litedb")
                .readonly(true)
                .connect()
                .unwrap();
    }
}

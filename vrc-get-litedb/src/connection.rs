use crate::lowlevel;
use super::Result;

pub struct DatabaseConnection {
    ptr: lowlevel::GcHandle,
}

impl DatabaseConnection {
    pub fn connect(string: &str) -> Result<DatabaseConnection> {
        unsafe {
            vrc_get_litedb_database_connection_new(lowlevel::FFISlice::from_byte_slice(string.as_bytes()))
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
    fn vrc_get_litedb_database_connection_new(string: lowlevel::FFISlice) -> super::error::HandleErrorResult;
    fn vrc_get_litedb_database_connection_dispose(ptr: isize);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connect() {
        let connection = DatabaseConnection::connect("vcc.litedb").unwrap();
    }
}

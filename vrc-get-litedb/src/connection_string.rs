use crate::connection::DatabaseConnection;
use crate::lowlevel;

/// Connection string for LiteDB
///
/// Rust representation of the `LiteDB.ConnectionString`.
/// This struct holds the values of the connections string and will be converted to 
/// `LiteDB.ConnectionString` when passed to the C# code.
pub struct ConnectionString<'a> {
    filename: &'a str,
    readonly: bool,
}

impl<'a> ConnectionString<'a> {
    pub fn connect(&self) -> crate::Result<DatabaseConnection> {
        DatabaseConnection::connect(self)
    }
}

impl<'a> ConnectionString<'a> {
    /// Create a new connection string
    pub fn new(filename: &'a str) -> Self {
        Self {
            filename,
            readonly: false,
        }
    }

    /// Set the connection string to read only
    pub fn readonly(&mut self, readonly: bool) -> &mut Self {
        self.readonly = readonly;
        self
    }
}

#[repr(C)]
pub(crate) struct ConnectionStringFFI {
    filename: lowlevel::FFISlice,
    readonly: bool,
}

impl<'a> ConnectionStringFFI {
    pub fn from(cs: &ConnectionString) -> Self {
        Self {
            filename: lowlevel::FFISlice::from_byte_slice(cs.filename.as_bytes()),
            readonly: cs.readonly,
        }
    }
}

pub mod connection;
mod connection_string; // exported in connection
mod error;
mod lowlevel;

pub use connection::ConnectionString;
pub use connection::DatabaseConnection;
pub use error::Error;
pub use error::ErrorKind;

pub type Result<T> = std::result::Result<T, Error>;

mod bson;
pub mod connection;
mod connection_string; // exported in connection
pub mod error;
mod lowlevel;
pub mod project;

pub type Result<T> = std::result::Result<T, error::Error>;

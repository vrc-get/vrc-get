mod bson;
mod connection;
mod connection_string;
mod error;
mod lowlevel;
mod project;

type Result<T> = std::result::Result<T, error::LiteDbError>;

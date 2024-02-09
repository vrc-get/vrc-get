mod bson;
pub mod connection;
mod connection_string; // exported in connection
mod error;
mod lowlevel;
mod project;
mod unity_version;

pub use bson::DateTime;
pub use bson::ObjectId;
pub use connection::ConnectionString;
pub use connection::DatabaseConnection;
pub use error::Error;
pub use error::ErrorKind;
pub use project::Project;
pub use project::ProjectType;
pub use unity_version::UnityVersion;

pub type Result<T> = std::result::Result<T, Error>;

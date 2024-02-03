mod lowlevel;
use std::path::Path;
use crate::bson::ObjectId;

mod connection;
mod error;
mod connection_string;
mod bson;

type Result<T> = std::result::Result<T, error::LiteDbError>;


#[repr(transparent)]
struct ProjectType(u32);

struct Project {
    path: Box<Path>,
    unity_version: Box<str>,
    created_at: u64,
    updated_at: u64,
    type_: ProjectType,
    id: ObjectId,
    favorite: bool,
}



mod lowlevel;
use std::path::Path;
mod connection;
mod error;
mod connection_string;

type Result<T> = std::result::Result<T, error::LiteDbError>;




#[repr(C)]
struct ObjectId {
    bytes: [u8; 12],
}

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



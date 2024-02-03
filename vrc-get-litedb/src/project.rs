use crate::bson::ObjectId;
use crate::lowlevel::FFISlice;

#[repr(transparent)]
#[derive(Debug)]
struct ProjectType(u32);

/// Represents a VCC Project
#[derive(Debug)]
pub struct Project {
    path: Box<str>,
    unity_version: Option<Box<str>>,
    created_at: u64, // milliseconds since Unix epoch in UTC
    updated_at: u64, // milliseconds since Unix epoch in UTC
    type_: ProjectType,
    id: ObjectId,
    favorite: bool,
}

#[repr(C)]
pub(crate) struct ProjectFFI {
    path: FFISlice,
    unity_version: FFISlice,
    created_at: u64, // milliseconds since Unix epoch in UTC
    updated_at: u64, // milliseconds since Unix epoch in UTC
    type_: ProjectType,
    id: ObjectId,
    favorite: u8,
}

impl Project {
    pub unsafe fn from_ffi(ffi: ProjectFFI) -> Self {
        Self {
            path: unsafe {
                std::str::from_boxed_utf8_unchecked(FFISlice::as_boxed_byte_slice(ffi.path))
            },
            unity_version: unsafe {
                FFISlice::as_boxed_byte_slice_option(ffi.unity_version)
                    .map(|x| std::str::from_boxed_utf8_unchecked(x))
            },
            created_at: ffi.created_at,
            updated_at: ffi.updated_at,
            type_: ffi.type_,
            id: ffi.id,
            favorite: ffi.favorite != 0,
        }
    }
}

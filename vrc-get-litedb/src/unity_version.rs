#![allow(dead_code)]
use crate::bson::ObjectId;
use crate::lowlevel::{FFISlice, FromFFI, ToFFI};
use std::fmt::Debug;

/// Represents a Unity Version on the PC
#[derive(Debug)]
pub struct UnityVersion {
    path: Box<str>,
    version: Option<Box<str>>,
    id: ObjectId,
    loaded_from_hub: bool,
}

impl UnityVersion {
    pub fn new(path: Box<str>, version: Box<str>, loaded_from_hub: bool) -> Self {
        Self {
            path,
            version: Some(version),
            loaded_from_hub,
            id: ObjectId::new(),
        }
    }

    pub fn id(&self) -> ObjectId {
        self.id
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    pub fn loaded_from_hub(&self) -> bool {
        self.loaded_from_hub
    }

    pub fn set_loaded_from_hub(&mut self, loaded_from_hub: bool) {
        self.loaded_from_hub = loaded_from_hub;
    }
}

#[repr(C)]
pub(crate) struct UnityVersionFFI {
    path: FFISlice,
    version: FFISlice,
    id: ObjectId,
    loaded_from_hub: u8,
}

impl FromFFI for UnityVersion {
    type FFIType = UnityVersionFFI;

    unsafe fn from_ffi(ffi: UnityVersionFFI) -> Self {
        Self {
            path: FromFFI::from_ffi(ffi.path),
            version: FromFFI::from_ffi(ffi.version),
            id: ffi.id,
            loaded_from_hub: ffi.loaded_from_hub != 0,
        }
    }
}

impl ToFFI for UnityVersion {
    type FFIType = UnityVersionFFI;

    unsafe fn to_ffi(&self) -> Self::FFIType {
        UnityVersionFFI {
            path: self.path.to_ffi(),
            version: self.version.as_deref().to_ffi(),
            id: self.id,
            loaded_from_hub: self.loaded_from_hub as u8,
        }
    }
}

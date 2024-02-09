use crate::lowlevel::{FFISlice, GcHandle};
use std::str;

#[derive(Debug)]
pub struct Error {
    message: Box<str>,
    code: ErrorKind,
}

impl Error {
    pub(crate) unsafe fn from_ffi(error: ErrorFFI) -> Self {
        let message = str::from_boxed_utf8_unchecked(error.message.into_boxed_byte_slice());
        if error.code == i32::MIN {
            // -1 means unexpected error in C# code so panic here
            panic!("{}", message);
        }

        Self {
            // SAFETY: C# guarantees the safety.
            code: std::mem::transmute(error.code),
            message,
        }
    }

    pub fn kind(&self) -> ErrorKind {
        self.code
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

#[cfg_attr(not(doc), repr(i32))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    NotFound = -1,
    PermissionDenied = -3,
    InvalidFilename = -4,
    OtherIO = -5,
    InvalidDatabase = 103,
    IndexDuplicateKey = 110,
    InvalidIndexKey = 111,
    IndexNotFound = 112,
    LockTimeout = 120,
    InvalidTransactionState = 126,
    InvalidCollectionName = 130,
    InvalidUpdateField = 136,
    UnexpectedToken = 203,
    InvalidDataType = 204,
    InvalidInitialSize = 211,
    InvalidNullCharString = 212,
    InvalidFreeSpacePage = 213,
    Unsupported = 300,
    Uncategorized = 400,
}

#[repr(C)]
pub(crate) struct ErrorFFI {
    // must be
    message: FFISlice<u8>,
    code: i32,
}

impl ErrorFFI {
    pub unsafe fn into_result(self) -> super::Result<()> {
        if self.code == 0 && self.message.is_null() {
            return Ok(());
        }
        Err(Error::from_ffi(self))
    }
}

#[repr(C)]
pub(crate) struct HandleErrorResult {
    pub result: Option<GcHandle>,
    pub error: ErrorFFI,
}

impl HandleErrorResult {
    pub unsafe fn into_result(self) -> super::Result<GcHandle> {
        if let Some(result) = self.result {
            Ok(result)
        } else {
            Err(Error::from_ffi(self.error))
        }
    }
}

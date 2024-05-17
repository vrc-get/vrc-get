use crate::lowlevel::{FFISlice, GcHandle};
use bson::de::Error as DeError;
use bson::ser::Error as SerError;
use std::io::Error as IoError;
use std::io::ErrorKind as IoErrorKind;
use std::str;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct CsError {
    message: Box<str>,
    code: ErrorKind,
}

impl CsError {
    pub(crate) unsafe fn from_ffi(error: ErrorFFI) -> Self {
        let message = str::from_boxed_utf8_unchecked(error.message.into_boxed_slice());
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

impl std::fmt::Display for CsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub struct Error {
    repr: ErrorRepr,
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        match &self.repr {
            ErrorRepr::Cs(e) => e.kind(),
            ErrorRepr::Ser(SerError::Io(ref io)) | ErrorRepr::De(DeError::Io(ref io)) => {
                match io.kind() {
                    IoErrorKind::NotFound => ErrorKind::NotFound,
                    IoErrorKind::PermissionDenied => ErrorKind::PermissionDenied,
                    IoErrorKind::InvalidInput => ErrorKind::InvalidFilename,
                    IoErrorKind::Other => ErrorKind::OtherIO,
                    IoErrorKind::InvalidData => ErrorKind::InvalidData,
                    _ => ErrorKind::OtherIO,
                }
            }
            ErrorRepr::Ser(_) => ErrorKind::InvalidData,
            ErrorRepr::De(_) => ErrorKind::InvalidData,
        }
    }
}

enum ErrorRepr {
    Cs(CsError),
    Ser(SerError),
    De(DeError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.repr {
            ErrorRepr::Cs(e) => write!(f, "{}", e),
            ErrorRepr::Ser(e) => write!(f, "{}", e),
            ErrorRepr::De(e) => write!(f, "{}", e),
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.repr {
            ErrorRepr::Cs(e) => f
                .debug_struct("CsError")
                .field("message", &e.message)
                .field("code", &e.code)
                .finish(),
            ErrorRepr::Ser(e) => f.debug_tuple("SerError").field(&e).finish(),
            ErrorRepr::De(e) => f.debug_tuple("DeError").field(&e).finish(),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.repr {
            ErrorRepr::Cs(_) => None,
            ErrorRepr::Ser(e) => Some(e),
            ErrorRepr::De(e) => Some(e),
        }
    }
}

impl From<Error> for IoError {
    fn from(value: Error) -> Self {
        match value.repr {
            ErrorRepr::Cs(ref cs) => IoError::new(cs.kind().into(), value),
            ErrorRepr::Ser(SerError::Io(io)) | ErrorRepr::De(DeError::Io(io)) => {
                Arc::try_unwrap(io).unwrap_or_else(|io| IoError::new(io.kind(), io))
            }
            ErrorRepr::Ser(ser) => IoError::new(IoErrorKind::InvalidData, ser),
            ErrorRepr::De(de) => IoError::new(IoErrorKind::InvalidData, de),
        }
    }
}

impl From<CsError> for Error {
    fn from(cs: CsError) -> Self {
        Error {
            repr: ErrorRepr::Cs(cs),
        }
    }
}

impl From<DeError> for Error {
    fn from(de: DeError) -> Self {
        Error {
            repr: ErrorRepr::De(de),
        }
    }
}

impl From<SerError> for Error {
    fn from(ser: SerError) -> Self {
        Error {
            repr: ErrorRepr::Ser(ser),
        }
    }
}

#[cfg_attr(not(doc), repr(i32))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    NotFound = -1,
    PermissionDenied = -3,
    InvalidFilename = -4,
    OtherIO = -5,
    InvalidData = -6,
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

impl From<ErrorKind> for std::io::ErrorKind {
    fn from(value: ErrorKind) -> Self {
        match value {
            ErrorKind::NotFound => IoErrorKind::NotFound,
            ErrorKind::PermissionDenied => IoErrorKind::PermissionDenied,
            ErrorKind::InvalidFilename => IoErrorKind::InvalidInput,
            ErrorKind::OtherIO => IoErrorKind::Other, // might have suitable error kind but not sure.
            ErrorKind::InvalidData => IoErrorKind::InvalidData, // for rust data errors
            ErrorKind::InvalidDatabase => IoErrorKind::InvalidData,
            ErrorKind::IndexDuplicateKey => IoErrorKind::InvalidData,
            ErrorKind::InvalidIndexKey => IoErrorKind::Unsupported,
            ErrorKind::IndexNotFound => IoErrorKind::InvalidInput,
            ErrorKind::LockTimeout => IoErrorKind::TimedOut,
            ErrorKind::InvalidTransactionState => IoErrorKind::InvalidInput,
            ErrorKind::InvalidCollectionName => IoErrorKind::InvalidInput,
            ErrorKind::InvalidUpdateField => IoErrorKind::InvalidInput,
            ErrorKind::UnexpectedToken => IoErrorKind::InvalidData,
            ErrorKind::InvalidDataType => IoErrorKind::InvalidData,
            ErrorKind::InvalidInitialSize => IoErrorKind::InvalidInput,
            ErrorKind::InvalidNullCharString => IoErrorKind::InvalidInput,
            ErrorKind::InvalidFreeSpacePage => IoErrorKind::InvalidData,
            ErrorKind::Unsupported => IoErrorKind::Unsupported,
            ErrorKind::Uncategorized => IoErrorKind::Other,
        }
    }
}

#[repr(C)]
pub(crate) struct ErrorFFI {
    // must be
    message: FFISlice<u8>,
    code: i32,
}

impl ErrorFFI {
    pub unsafe fn into_result(self) -> Result<(), CsError> {
        if self.code == 0 && self.message.is_null() {
            return Ok(());
        }
        Err(CsError::from_ffi(self))
    }
}

#[repr(C)]
pub(crate) struct HandleErrorResult {
    pub result: Option<GcHandle>,
    pub error: ErrorFFI,
}

impl HandleErrorResult {
    pub unsafe fn into_result(self) -> Result<GcHandle, CsError> {
        if let Some(result) = self.result {
            Ok(result)
        } else {
            Err(CsError::from_ffi(self.error))
        }
    }
}

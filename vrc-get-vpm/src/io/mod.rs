use std::ffi::OsString;
use std::future::Future;
use std::path::Path;

pub(crate) use futures::Stream;
pub(crate) use futures::io::{
    AsyncRead, AsyncSeek, AsyncWrite, BufReader, Error, ErrorKind, Result, copy, empty, sink,
};
pub(crate) use std::io::SeekFrom;

mod tokio;
pub use tokio::DefaultEnvironmentIo;
pub use tokio::DefaultProjectIo;
pub use tokio::DirEntry as TokioDirEntry;
pub use tokio::File as TokioFile;

pub trait IoTrait: Sync {
    fn create_dir_all(&self, path: &Path) -> impl Future<Output = Result<()>> + Send;
    fn write(&self, path: &Path, content: &[u8]) -> impl Future<Output = Result<()>> + Send;
    fn write_sync(&self, path: &Path, content: &[u8]) -> impl Future<Output = Result<()>> + Send;
    fn remove_file(&self, path: &Path) -> impl Future<Output = Result<()>> + Send;
    fn remove_dir(&self, path: &Path) -> impl Future<Output = Result<()>> + Send;
    fn remove_dir_all(&self, path: &Path) -> impl Future<Output = Result<()>> + Send;
    fn rename(&self, from: &Path, to: &Path) -> impl Future<Output = Result<()>> + Send;
    fn metadata(&self, path: &Path) -> impl Future<Output = Result<Metadata>> + Send;

    type DirEntry: DirEntry;
    type ReadDirStream: Stream<Item = Result<Self::DirEntry>> + Unpin + Send;

    fn is_file(&self, path: &Path) -> impl Future<Output = bool> + Send {
        async {
            self.metadata(path)
                .await
                .map(|x| x.is_file())
                .unwrap_or(false)
        }
    }

    fn is_dir(&self, path: &Path) -> impl Future<Output = bool> + Send {
        async {
            self.metadata(path)
                .await
                .map(|x| x.is_dir())
                .unwrap_or(false)
        }
    }

    fn read_dir(&self, path: &Path) -> impl Future<Output = Result<Self::ReadDirStream>> + Send;

    type FileStream: FileStream;

    fn create_new(&self, path: &Path) -> impl Future<Output = Result<Self::FileStream>> + Send;
    fn create(&self, path: &Path) -> impl Future<Output = Result<Self::FileStream>> + Send;
    fn open(&self, path: &Path) -> impl Future<Output = Result<Self::FileStream>> + Send;
}

pub trait FileStream: AsyncRead + AsyncWrite + AsyncSeek + Unpin + Send {}

#[derive(Debug, Copy, Clone)]
pub struct FileType {
    is_file: bool,
    is_dir: bool,
}

impl FileType {
    pub fn file() -> Self {
        Self {
            is_file: true,
            is_dir: false,
        }
    }

    pub fn dir() -> Self {
        Self {
            is_file: false,
            is_dir: true,
        }
    }

    pub fn is_file(&self) -> bool {
        self.is_file
    }

    pub fn is_dir(&self) -> bool {
        self.is_dir
    }
}

impl From<std::fs::FileType> for FileType {
    fn from(value: std::fs::FileType) -> Self {
        Self {
            is_dir: value.is_dir(),
            is_file: value.is_file(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Metadata {
    file_type: FileType,
}

impl Metadata {
    pub fn file() -> Self {
        Self {
            file_type: FileType::file(),
        }
    }

    pub fn dir() -> Self {
        Self {
            file_type: FileType::dir(),
        }
    }

    pub fn file_type(&self) -> FileType {
        self.file_type
    }

    pub fn is_file(&self) -> bool {
        self.file_type.is_file
    }

    pub fn is_dir(&self) -> bool {
        self.file_type.is_dir
    }
}

impl From<std::fs::Metadata> for Metadata {
    fn from(value: std::fs::Metadata) -> Self {
        Self {
            file_type: value.file_type().into(),
        }
    }
}

#[derive(Debug)]
pub struct ExitStatus {
    inner: ExitStatusEnum,
}

#[derive(Debug)]
enum ExitStatusEnum {
    Std(std::process::ExitStatus),
    Custom { success: bool },
}

impl ExitStatus {
    pub fn new(success: bool) -> Self {
        Self {
            inner: ExitStatusEnum::Custom { success },
        }
    }

    pub fn success(&self) -> bool {
        match self.inner {
            ExitStatusEnum::Std(std) => std.success(),
            ExitStatusEnum::Custom { success, .. } => success,
        }
    }
}

impl From<std::process::ExitStatus> for ExitStatus {
    fn from(value: std::process::ExitStatus) -> Self {
        Self {
            inner: ExitStatusEnum::Std(value),
        }
    }
}

impl std::fmt::Display for ExitStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            ExitStatusEnum::Std(std) => std::fmt::Display::fmt(std, f),
            ExitStatusEnum::Custom { success: true, .. } => f.write_str("exits successfully"),
            ExitStatusEnum::Custom { success: false, .. } => f.write_str("exits non successfully"),
        }
    }
}

pub trait DirEntry {
    fn file_name(&self) -> OsString;
    fn file_type(&self) -> impl Future<Output = Result<FileType>> + Send;
    fn metadata(&self) -> impl Future<Output = Result<Metadata>> + Send;
}

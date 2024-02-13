use std::ffi::{OsStr, OsString};
use std::future::Future;
use std::path::{Path, PathBuf};

pub(crate) use futures::io::{
    copy, empty, sink, AsyncRead, AsyncSeek, AsyncWrite, BufReader, Error, ErrorKind, Result,
};
pub(crate) use futures::Stream;
pub(crate) use std::io::SeekFrom;

#[cfg(feature = "tokio")]
mod tokio;
#[cfg(feature = "tokio")]
pub use tokio::DefaultEnvironmentIo;
#[cfg(feature = "tokio")]
pub use tokio::DefaultProjectIo;

/// Wrapper for the file system operation for the Environment
///
/// All relative paths should be resolved as a relative path from the environment folder.
/// Which is `%APPDATA%\\VRChatCreatorCompanion` or `${XDG_DATA_HOME}/VRChatCreatorCompanion` by default.
pub trait EnvironmentIo: Sync + IoTrait {
    /// We may need to resolve a relative path to an absolute path for some reason.
    /// For example, to get the absolute path of the Repos folder for creating local cache and cleanup repos folder.
    fn resolve(&self, path: &Path) -> PathBuf;
    #[cfg(feature = "vrc-get-litedb")]
    fn connect_lite_db(&self) -> Result<vrc_get_litedb::DatabaseConnection>;
    #[cfg(feature = "experimental-project-management")]
    type ProjectIo: ProjectIo;

    #[cfg(feature = "experimental-project-management")]
    fn new_project_io(&self, path: &Path) -> Self::ProjectIo;
}

/// Wrapper for the file system operation for the [UnityProject]
///
/// Absolute paths are not allowed and relative paths should be resolved as a relative path from the project folder.
///
/// [UnityProject]: crate::unity_project::UnityProject
pub trait ProjectIo: Sync + IoTrait {}

pub trait FileSystemProjectIo {
    fn location(&self) -> &Path;
}

pub trait IoTrait: Sync {
    fn create_dir_all(&self, path: &Path) -> impl Future<Output = Result<()>> + Send;
    fn write(&self, path: &Path, content: &[u8]) -> impl Future<Output = Result<()>> + Send;
    fn remove_file(&self, path: &Path) -> impl Future<Output = Result<()>> + Send;
    fn remove_dir_all(&self, path: &Path) -> impl Future<Output = Result<()>> + Send;
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

    type FileStream: AsyncRead + AsyncWrite + AsyncSeek + Unpin + Send;

    fn create_new(&self, path: &Path) -> impl Future<Output = Result<Self::FileStream>> + Send;
    fn create(&self, path: &Path) -> impl Future<Output = Result<Self::FileStream>> + Send;
    fn open(&self, path: &Path) -> impl Future<Output = Result<Self::FileStream>> + Send;

    // simple process operation.
    fn command_status(
        &self,
        command: &OsStr,
        args: &[&OsStr],
    ) -> impl Future<Output = Result<ExitStatus>> + Send;
    fn command_output(
        &self,
        command: &OsStr,
        args: &[&OsStr],
    ) -> impl Future<Output = Result<Output>> + Send;
}

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

#[derive(Debug)]
pub struct Output {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl From<std::process::Output> for Output {
    fn from(value: std::process::Output) -> Self {
        Self {
            status: value.status.into(),
            stdout: value.stdout,
            stderr: value.stderr,
        }
    }
}

pub trait DirEntry {
    fn file_name(&self) -> OsString;
    fn file_type(&self) -> impl Future<Output = Result<FileType>> + Send;
    fn metadata(&self) -> impl Future<Output = Result<Metadata>> + Send;
}

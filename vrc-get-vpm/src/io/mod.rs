use std::ffi::{OsStr, OsString};
use std::future::Future;
use std::path::{Path, PathBuf};

pub(crate) use futures::io::{
    copy, empty, sink, AsyncRead, AsyncSeek, AsyncWrite, BufReader, Error, ErrorKind, Result,
};
pub(crate) use futures::Stream;
pub(crate) use std::io::SeekFrom;
pub(crate) use std::process::ExitStatus;
pub(crate) use std::process::Output;

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

pub trait IoTrait {
    fn create_dir_all(&self, path: &Path) -> impl Future<Output = Result<()>> + Send;
    fn write(&self, path: &Path, content: &[u8]) -> impl Future<Output = Result<()>> + Send;
    fn remove_file(&self, path: &Path) -> impl Future<Output = Result<()>> + Send;
    fn remove_dir_all(&self, path: &Path) -> impl Future<Output = Result<()>> + Send;
    fn metadata(&self, path: &Path) -> impl Future<Output = Result<std::fs::Metadata>> + Send;

    type DirEntry: DirEntry;
    type ReadDirStream: Stream<Item = Result<Self::DirEntry>> + Unpin + Send;

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

pub trait DirEntry {
    fn file_name(&self) -> OsString;
    fn file_type(&self) -> impl Future<Output = Result<std::fs::FileType>> + Send;
    fn metadata(&self) -> impl Future<Output = Result<std::fs::Metadata>> + Send;
}

use crate::io;
use std::ffi::OsString;
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
pub trait EnvironmentIo: crate::traits::seal::Sealed + Sync + IoTrait {
    fn resolve(&self, path: impl AsRef<Path>) -> PathBuf;
}

/// Wrapper for the file system operation for the [UnityProject]
///
/// Absolute paths are not allowed and relative paths should be resolved as a relative path from the project folder.
///
/// [UnityProject]: crate::unity_project::UnityProject
pub trait ProjectIo: crate::traits::seal::Sealed + Sync + IoTrait {}

pub trait FileSystemProjectIo {
    fn location(&self) -> &Path;
}

pub enum SymlinkKind {
    File,
    Directory,
}

pub trait IoTrait {
    fn create_dir_all(&self, path: impl AsRef<Path>)
        -> impl Future<Output = io::Result<()>> + Send;
    fn write(
        &self,
        path: impl AsRef<Path>,
        content: impl AsRef<[u8]>,
    ) -> impl Future<Output = io::Result<()>> + Send;
    fn remove_file(&self, path: impl AsRef<Path>) -> impl Future<Output = io::Result<()>> + Send;
    fn remove_dir_all(&self, path: impl AsRef<Path>)
        -> impl Future<Output = io::Result<()>> + Send;
    fn symlink(
        &self,
        path: impl AsRef<Path>,
        kind: Option<SymlinkKind>,
        link_target: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<()>> + Send;
    fn read_symlink(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<(PathBuf, Option<SymlinkKind>)>> + Send;
    fn metadata(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<std::fs::Metadata>> + Send;

    type DirEntry: DirEntry;
    type ReadDirStream: Stream<Item = io::Result<Self::DirEntry>> + Unpin + Send;

    fn read_dir(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<Self::ReadDirStream>> + Send;

    type FileStream: AsyncRead + AsyncWrite + AsyncSeek + Unpin + Send;

    fn create_new(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<Self::FileStream>> + Send;
    fn create(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<Self::FileStream>> + Send;
    fn open(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<Self::FileStream>> + Send;
}

pub trait DirEntry {
    fn file_name(&self) -> OsString;
    fn file_type(&self) -> impl Future<Output = io::Result<std::fs::FileType>> + Send;
    fn metadata(&self) -> impl Future<Output = io::Result<std::fs::Metadata>> + Send;
}

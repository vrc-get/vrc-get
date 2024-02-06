mod tokio;
use futures::{AsyncRead, AsyncSeek, AsyncWrite, Stream};
use std::ffi::OsString;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
pub(crate) use tokio::DefaultEnvironmentIo;
pub(crate) use tokio::DefaultProjectIo;

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
    fn path(&self) -> PathBuf;
    fn file_name(&self) -> OsString;
    fn metadata(&self) -> impl Future<Output = io::Result<std::fs::Metadata>> + Send;
}

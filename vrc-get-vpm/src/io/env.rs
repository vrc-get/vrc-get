use futures::{AsyncRead, AsyncSeek, AsyncWrite, Stream};
use std::ffi::OsString;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};

/// Wrapper for the file system operation for the Environment
///
/// All relative paths should be resolved as a relative path from the environment folder.
/// Which is `%APPDATA%\\VRChatCreatorCompanion` or `${XDG_DATA_HOME}/VRChatCreatorCompanion` by default.
pub trait EnvironmentIo: crate::traits::seal::Sealed + Sync + IoTrait {
    fn resolve(&self, path: impl AsRef<Path>) -> PathBuf;
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

#[derive(Debug)]
pub struct DefaultEnvironmentIo {
    root: Box<Path>,
}

impl DefaultEnvironmentIo {
    pub fn new(root: Box<Path>) -> Self {
        Self { root }
    }
}

impl crate::traits::seal::Sealed for DefaultEnvironmentIo {}

mod tokio {
    use crate::io::env::IoTrait;
    use crate::io::{DefaultEnvironmentIo, EnvironmentIo};
    use futures::{Stream, TryFutureExt};
    use std::ffi::OsString;
    use std::fs::Metadata;
    use std::future::Future;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::fs;
    use tokio_util::compat::TokioAsyncReadCompatExt;

    impl EnvironmentIo for DefaultEnvironmentIo {
        fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
            self.root.join(path)
        }
    }

    impl IoTrait for DefaultEnvironmentIo {
        fn create_dir_all(
            &self,
            path: impl AsRef<Path>,
        ) -> impl Future<Output = io::Result<()>> + Send {
            fs::create_dir_all(self.resolve(path))
        }

        fn write(
            &self,
            path: impl AsRef<Path>,
            content: impl AsRef<[u8]>,
        ) -> impl Future<Output = io::Result<()>> + Send {
            let path = self.resolve(path);
            let content = content.as_ref().to_owned();
            tokio::fs::write(path, content)
        }

        fn remove_file(
            &self,
            path: impl AsRef<Path>,
        ) -> impl Future<Output = io::Result<()>> + Send {
            fs::remove_file(self.resolve(path))
        }

        fn metadata(
            &self,
            path: impl AsRef<Path>,
        ) -> impl Future<Output = io::Result<Metadata>> + Send {
            fs::metadata(self.resolve(path))
        }

        type DirEntry = DirEntry;
        type ReadDirStream = ReadDir;

        fn read_dir(
            &self,
            path: impl AsRef<Path>,
        ) -> impl Future<Output = io::Result<Self::ReadDirStream>> + Send {
            fs::read_dir(self.resolve(path)).map_ok(ReadDir::new)
        }

        type FileStream = tokio_util::compat::Compat<fs::File>;

        fn create_new(
            &self,
            path: impl AsRef<Path>,
        ) -> impl Future<Output = io::Result<Self::FileStream>> + Send {
            let path = self.resolve(path);
            let mut options = fs::OpenOptions::new();
            options.create_new(true).write(true).read(true);
            async move {
                options
                    .open(path)
                    .and_then(|file| async { Ok(file.compat()) })
                    .await
            }
        }

        fn create(
            &self,
            path: impl AsRef<Path>,
        ) -> impl Future<Output = io::Result<Self::FileStream>> + Send {
            let path = self.resolve(path);
            let mut options = fs::OpenOptions::new();
            options.create(true).write(true).read(true);
            async move {
                options
                    .open(path)
                    .and_then(|file| async { Ok(file.compat()) })
                    .await
            }
        }

        fn open(
            &self,
            path: impl AsRef<Path>,
        ) -> impl Future<Output = io::Result<Self::FileStream>> + Send {
            let path = self.resolve(path);
            fs::File::open(path).and_then(|file| async { Ok(file.compat()) })
        }
    }

    pub struct ReadDir {
        inner: fs::ReadDir,
    }

    impl ReadDir {
        pub fn new(inner: fs::ReadDir) -> Self {
            Self { inner }
        }
    }

    impl Stream for ReadDir {
        type Item = io::Result<DirEntry>;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            match self.inner.poll_next_entry(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
                Poll::Ready(Ok(None)) => Poll::Ready(None),
                Poll::Ready(Ok(Some(entry))) => Poll::Ready(Some(Ok(DirEntry::new(entry)))),
            }
        }
    }

    pub struct DirEntry {
        inner: fs::DirEntry,
    }

    impl DirEntry {
        pub fn new(inner: fs::DirEntry) -> Self {
            Self { inner }
        }
    }

    impl super::DirEntry for DirEntry {
        fn path(&self) -> PathBuf {
            self.inner.path()
        }

        fn file_name(&self) -> OsString {
            self.inner.file_name()
        }

        fn metadata(&self) -> impl Future<Output = io::Result<Metadata>> + Send {
            self.inner.metadata()
        }
    }
}

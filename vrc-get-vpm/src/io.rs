use futures::{AsyncRead, AsyncSeek, AsyncWrite};
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};

/// Wrapper for the file system operation for the Environment
///
/// All relative paths should be resolved as a relative path from the environment folder.
/// Which is `%APPDATA%\\VRChatCreatorCompanion` or `${XDG_DATA_HOME}/VRChatCreatorCompanion` by default.
pub trait EnvironmentIo: crate::traits::seal::Sealed {
    fn resolve(&self, path: impl AsRef<Path>) -> PathBuf;

    fn create_dir_all(&self, path: impl AsRef<Path>) -> impl Future<Output = io::Result<()>>;
    fn write(
        &self,
        path: impl AsRef<Path>,
        content: impl AsRef<[u8]>,
    ) -> impl Future<Output = io::Result<()>>;

    type FileStream: AsyncRead + AsyncWrite + AsyncSeek + Unpin;

    fn create_new(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<Self::FileStream>>;
    fn create(&self, path: impl AsRef<Path>) -> impl Future<Output = io::Result<Self::FileStream>>;
    fn open(&self, path: impl AsRef<Path>) -> impl Future<Output = io::Result<Self::FileStream>>;
}

#[derive(Debug)]
pub(crate) struct DefaultEnvironmentIo {
    root: Box<Path>,
}

impl DefaultEnvironmentIo {
    pub fn new(root: Box<Path>) -> Self {
        Self { root }
    }
}

impl crate::traits::seal::Sealed for DefaultEnvironmentIo {}

mod tokio {
    use crate::io::{DefaultEnvironmentIo, EnvironmentIo};
    use std::io;
    use std::path::{Path, PathBuf};
    use tokio::fs;
    use tokio_util::compat::TokioAsyncReadCompatExt;

    impl EnvironmentIo for DefaultEnvironmentIo {
        fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
            self.root.join(path)
        }

        async fn create_dir_all(&self, path: impl AsRef<Path>) -> io::Result<()> {
            fs::create_dir_all(self.resolve(path)).await
        }

        async fn write(&self, path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> io::Result<()> {
            let path = self.resolve(path);
            let content = content.as_ref();
            tokio::fs::write(path, content).await
        }

        type FileStream = tokio_util::compat::Compat<fs::File>;

        async fn create_new(&self, path: impl AsRef<Path>) -> io::Result<Self::FileStream> {
            let path = self.resolve(path);
            fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .read(true)
                .open(path)
                .await
                .map(|file| file.compat())
        }

        async fn create(&self, path: impl AsRef<Path>) -> io::Result<Self::FileStream> {
            let path = self.resolve(path);
            fs::OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .open(path)
                .await
                .map(|file| file.compat())
        }

        async fn open(&self, path: impl AsRef<Path>) -> io::Result<Self::FileStream> {
            let path = self.resolve(path);
            fs::File::open(path).await.map(|file| file.compat())
        }
    }
}

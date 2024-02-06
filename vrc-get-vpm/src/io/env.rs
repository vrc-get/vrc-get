use futures::{AsyncRead, AsyncSeek, AsyncWrite};
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};

/// Wrapper for the file system operation for the Environment
///
/// All relative paths should be resolved as a relative path from the environment folder.
/// Which is `%APPDATA%\\VRChatCreatorCompanion` or `${XDG_DATA_HOME}/VRChatCreatorCompanion` by default.
pub trait EnvironmentIo: crate::traits::seal::Sealed + Sync {
    fn resolve(&self, path: impl AsRef<Path>) -> PathBuf;

    fn create_dir_all(&self, path: impl AsRef<Path>)
        -> impl Future<Output = io::Result<()>> + Send;
    fn write(
        &self,
        path: impl AsRef<Path>,
        content: impl AsRef<[u8]>,
    ) -> impl Future<Output = io::Result<()>> + Send;

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
    use crate::io::{DefaultEnvironmentIo, EnvironmentIo};
    use futures::TryFutureExt;
    use std::future::Future;
    use std::io;
    use std::path::{Path, PathBuf};
    use tokio::fs;
    use tokio_util::compat::TokioAsyncReadCompatExt;

    impl EnvironmentIo for DefaultEnvironmentIo {
        fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
            self.root.join(path)
        }

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
}

use crate::io::{EnvironmentIo, IoTrait, ProjectIo};
use futures::{Stream, TryFutureExt};
use std::ffi::OsString;
use std::fs::Metadata;
use std::future::Future;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs;
use tokio_util::compat::TokioAsyncReadCompatExt;

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

impl EnvironmentIo for DefaultEnvironmentIo {
    fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
        self.root.join(path)
    }
}

impl TokioIoTraitImpl for DefaultEnvironmentIo {
    fn resolve(&self, path: impl AsRef<Path>) -> io::Result<PathBuf> {
        Ok(self.root.join(path))
    }
}

#[derive(Debug)]
pub struct DefaultProjectIo {
    root: Box<Path>,
}

impl DefaultProjectIo {
    pub fn new(root: Box<Path>) -> Self {
        Self { root }
    }

    pub fn location(&self) -> &Path {
        &self.root
    }
}

impl crate::traits::seal::Sealed for DefaultProjectIo {}

impl ProjectIo for DefaultProjectIo {}

impl TokioIoTraitImpl for DefaultProjectIo {
    fn resolve(&self, path: impl AsRef<Path>) -> io::Result<PathBuf> {
        if path.as_ref().is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "absolute path is not allowed",
            ));
        }
        Ok(self.root.join(path))
    }
}

trait TokioIoTraitImpl {
    fn resolve(&self, path: impl AsRef<Path>) -> io::Result<PathBuf>;
}

macro_rules! resolved {
    ($self: ident: $path: ident => $expr: expr) => {{
        let resolved = $self.resolve($path);
        async move {
            match resolved {
                Ok($path) => $expr.await,
                Err(err) => Err(err),
            }
        }
    }};
}

impl<T: TokioIoTraitImpl> IoTrait for T {
    fn create_dir_all(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<()>> + Send {
        resolved!(self: path => fs::create_dir_all(path))
    }

    fn write(
        &self,
        path: impl AsRef<Path>,
        content: impl AsRef<[u8]>,
    ) -> impl Future<Output = io::Result<()>> + Send {
        let content = content.as_ref().to_owned();
        resolved!(self: path => tokio::fs::write(path, content))
    }

    fn remove_file(&self, path: impl AsRef<Path>) -> impl Future<Output = io::Result<()>> + Send {
        resolved!(self: path => fs::remove_file(path))
    }

    fn metadata(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<Metadata>> + Send {
        resolved!(self: path => fs::metadata(path))
    }

    type DirEntry = DirEntry;
    type ReadDirStream = ReadDir;

    fn read_dir(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<Self::ReadDirStream>> + Send {
        resolved!(self: path => fs::read_dir(path).map_ok(ReadDir::new))
    }

    type FileStream = tokio_util::compat::Compat<fs::File>;

    fn create_new(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<Self::FileStream>> + Send {
        resolved!(self: path => {
            let mut options = fs::OpenOptions::new();
            options.create_new(true).write(true).read(true);
            async move {
                options.open(path).map_ok(|file| file.compat()).await
            }
        })
    }

    fn create(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<Self::FileStream>> + Send {
        resolved!(self: path => {
            let mut options = fs::OpenOptions::new();
            options.create(true).write(true).read(true);
            async move {
                options
                    .open(path)
                    .and_then(|file| async { Ok(file.compat()) })
                    .await
            }
        })
    }

    fn open(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<Self::FileStream>> + Send {
        resolved!(self: path => fs::File::open(path).and_then(|file| async { Ok(file.compat()) }))
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

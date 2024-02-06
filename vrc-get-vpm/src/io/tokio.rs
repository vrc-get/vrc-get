use crate::io::{EnvironmentIo, IoTrait, ProjectIo, SymlinkKind};
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

    fn remove_dir_all(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<()>> + Send {
        resolved!(self: path => fs::remove_dir_all(path))
    }

    #[cfg(unix)]
    fn symlink(
        &self,
        path: impl AsRef<Path>,
        _kind: Option<SymlinkKind>,
        link_target: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<()>> + Send {
        let link_target = link_target.as_ref().to_owned();
        resolved!(self: path => fs::symlink(path, link_target))
    }

    #[cfg(unix)]
    fn read_symlink(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<(PathBuf, Option<SymlinkKind>)>> + Send {
        resolved!(self: path => async move {
            Ok((fs::read_link(path).await?, None))
        })
    }

    #[cfg(windows)]
    fn symlink(
        &self,
        path: impl AsRef<Path>,
        kind: Option<SymlinkKind>,
        link_target: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<()>> + Send {
        let link_target = link_target.as_ref().to_owned();
        resolved!(self: path => async move {
            match kind {
                Some(SymlinkKind::File) => tokio::fs::symlink_file(path, link_target).await,
                Some(SymlinkKind::Directory) => tokio::fs::symlink_dir(path, link_target).await,
                None => Err(io::Error::new(io::ErrorKind::InvalidInput, "symlink kind is required")),
            }
        })
    }

    #[cfg(windows)]
    fn read_symlink(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<(PathBuf, Option<SymlinkKind>)>> + Send {
        use std::os::windows::fs::FileTypeExt;
        resolved!(self: path => async move {
            let link = fs::read_link(path).await?;
            let file_type = fs::metadata(&link).await?;

            let kind = {
                if file_type.file_type().is_symlink_file() {
                    Some(SymlinkKind::File)
                } else if file_type.file_type().is_symlink_dir() {
                    Some(SymlinkKind::Directory)
                } else {
                    None
                }
            };
            Ok((link, kind))
        })
    }

    #[cfg(not(any(unix, windows)))]
    fn symlink(
        &self,
        path: impl AsRef<Path>,
        kind: Option<SymlinkKind>,
        link_target: impl AsRef<Path>,
    ) -> impl Future<Output = io::Result<()>> + Send {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "platform without symlink detected",
        ));
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

    fn file_type(&self) -> impl Future<Output = io::Result<std::fs::FileType>> + Send {
        self.inner.file_type()
    }

    fn metadata(&self) -> impl Future<Output = io::Result<Metadata>> + Send {
        self.inner.metadata()
    }
}

use crate::io;
use crate::io::{EnvironmentIo, FileSystemProjectIo, IoTrait, ProjectIo, SymlinkKind};
use futures::{Stream, TryFutureExt};
use log::debug;
use std::ffi::OsString;
use std::fs::Metadata;
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

    pub fn new_default() -> Self {
        let mut folder = Self::get_local_config_folder();
        folder.push("VRChatCreatorCompanion");
        let folder = folder;

        debug!(
            "initializing EnvironmentIo with config folder {}",
            folder.display()
        );

        DefaultEnvironmentIo::new(folder.clone().into_boxed_path())
    }

    #[cfg(windows)]
    fn get_local_config_folder() -> PathBuf {
        return dirs_sys::known_folder_local_app_data().expect("LocalAppData not found");
    }

    #[cfg(not(windows))]
    fn get_local_config_folder() -> PathBuf {
        if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
            debug!("XDG_DATA_HOME found {:?}", data_home);
            return data_home.into();
        }

        // fallback: use HOME
        if let Some(home_folder) = std::env::var_os("HOME") {
            debug!("HOME found {:?}", home_folder);
            let mut path = PathBuf::from(home_folder);
            path.push(".local/share");
            return path;
        }

        panic!("no XDG_DATA_HOME nor HOME are set!")
    }
}

impl crate::traits::seal::Sealed for DefaultEnvironmentIo {}

impl EnvironmentIo for DefaultEnvironmentIo {
    fn resolve(&self, path: &Path) -> PathBuf {
        self.root.join(path)
    }
}

impl TokioIoTraitImpl for DefaultEnvironmentIo {
    fn resolve(&self, path: &Path) -> io::Result<PathBuf> {
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

    pub fn find_project_parent(path_buf: PathBuf) -> io::Result<Self> {
        Self::find_unity_project_path(path_buf).map(Self::new)
    }

    fn find_unity_project_path(mut candidate: PathBuf) -> io::Result<Box<Path>> {
        loop {
            candidate.push("Packages");
            candidate.push("vpm-manifest.json");

            if candidate.exists() {
                debug!("vpm-manifest.json found at {}", candidate.display());
                // if there's vpm-manifest.json, it's a project path
                candidate.pop();
                candidate.pop();
                return Ok(candidate.into_boxed_path());
            }

            // replace vpm-manifest.json -> manifest.json
            candidate.pop();
            candidate.push("manifest.json");

            if candidate.exists() {
                debug!("manifest.json found at {}", candidate.display());
                // if there's manifest.json (which is manifest of UPM), it's a project path
                candidate.pop();
                candidate.pop();
                return Ok(candidate.into_boxed_path());
            }

            // remove Packages/manifest.json
            candidate.pop();
            candidate.pop();

            debug!("Unity Project not found on {}", candidate.display());

            // go to parent dir
            if !candidate.pop() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Unity project Not Found",
                ));
            }
        }
    }
}

impl crate::traits::seal::Sealed for DefaultProjectIo {}

impl ProjectIo for DefaultProjectIo {}

impl FileSystemProjectIo for DefaultProjectIo {
    fn location(&self) -> &Path {
        &self.root
    }
}

impl TokioIoTraitImpl for DefaultProjectIo {
    fn resolve(&self, path: &Path) -> io::Result<PathBuf> {
        if path.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "absolute path is not allowed",
            ));
        }
        Ok(self.root.join(path))
    }
}

trait TokioIoTraitImpl {
    fn resolve(&self, path: &Path) -> io::Result<PathBuf>;
}

impl<T: TokioIoTraitImpl + Sync> IoTrait for T {
    async fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(self.resolve(path)?).await
    }

    async fn write(&self, path: &Path, content: &[u8]) -> io::Result<()> {
        tokio::fs::write(self.resolve(path)?, content).await
    }

    async fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(self.resolve(path)?).await
    }

    async fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir_all(self.resolve(path)?).await
    }

    #[cfg(unix)]
    async fn symlink(
        &self,
        path: &Path,
        _kind: Option<SymlinkKind>,
        link_target: &Path,
    ) -> io::Result<()> {
        fs::symlink(self.resolve(path)?, link_target).await
    }

    #[cfg(unix)]
    async fn read_symlink(&self, path: &Path) -> io::Result<(PathBuf, Option<SymlinkKind>)> {
        Ok((fs::read_link(self.resolve(path)?).await?, None))
    }

    #[cfg(windows)]
    async fn symlink(
        &self,
        path: &Path,
        kind: Option<SymlinkKind>,
        link_target: &Path,
    ) -> io::Result<()> {
        let path = self.resolve(path)?;
        match kind {
            Some(SymlinkKind::File) => tokio::fs::symlink_file(path, link_target).await,
            Some(SymlinkKind::Directory) => tokio::fs::symlink_dir(path, link_target).await,
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "symlink kind is required",
            )),
        }
    }

    #[cfg(windows)]
    async fn read_symlink(&self, path: &Path) -> io::Result<(PathBuf, Option<SymlinkKind>)> {
        use std::os::windows::fs::FileTypeExt;
        let path = self.resolve(path)?;

        let link = fs::read_link(&path).await?;
        let file_type = fs::metadata(&path).await?;

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
    }

    #[cfg(not(any(unix, windows)))]
    async fn symlink(
        &self,
        path: &Path,
        kind: Option<SymlinkKind>,
        link_target: &Path,
    ) -> io::Result<()> {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "platform without symlink detected",
        ));
    }

    async fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        fs::metadata(self.resolve(path)?).await
    }

    type DirEntry = DirEntry;
    type ReadDirStream = ReadDir;

    async fn read_dir(&self, path: &Path) -> io::Result<Self::ReadDirStream> {
        Ok(ReadDir::new(fs::read_dir(self.resolve(path)?).await?))
    }

    type FileStream = tokio_util::compat::Compat<fs::File>;

    async fn create_new(&self, path: &Path) -> io::Result<Self::FileStream> {
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .read(true)
            .open(self.resolve(path)?)
            .map_ok(|file| file.compat())
            .await
    }

    async fn create(&self, path: &Path) -> io::Result<Self::FileStream> {
        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(self.resolve(path)?)
            .and_then(|file| async { Ok(file.compat()) })
            .await
    }

    async fn open(&self, path: &Path) -> io::Result<Self::FileStream> {
        Ok(fs::File::open(self.resolve(path)?).await?.compat())
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
    fn file_name(&self) -> OsString {
        self.inner.file_name()
    }

    async fn file_type(&self) -> io::Result<std::fs::FileType> {
        self.inner.file_type().await
    }

    async fn metadata(&self) -> io::Result<Metadata> {
        self.inner.metadata().await
    }
}

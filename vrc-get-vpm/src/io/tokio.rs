use crate::io;
use crate::io::{FileStream, FileType, IoTrait, Metadata};
use futures::{Stream, TryFutureExt};
use log::debug;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio_util::compat::TokioAsyncReadCompatExt;

#[derive(Debug, Clone)]
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
        dirs_sys::known_folder_local_app_data().expect("LocalAppData not found")
    }

    #[cfg(not(windows))]
    fn get_local_config_folder() -> PathBuf {
        if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
            debug!("XDG_DATA_HOME found {data_home:?}");
            return data_home.into();
        }

        // fallback: use HOME
        if let Some(home_folder) = std::env::var_os("HOME") {
            debug!("HOME found {home_folder:?}");
            let mut path = PathBuf::from(home_folder);
            path.push(".local/share");
            return path;
        }

        panic!("no XDG_DATA_HOME nor HOME are set!")
    }

    #[inline]
    pub fn resolve(&self, path: &Path) -> PathBuf {
        self.root.join(path)
    }

    #[allow(dead_code)]
    pub fn new_project_io(&self, path: &Path) -> DefaultProjectIo {
        DefaultProjectIo::new(path.into())
    }

    #[cfg(feature = "vrc-get-litedb")]
    #[allow(unused_variables)]
    pub async fn new_mutex(&self, lock_name: &OsStr) -> io::Result<impl Send + Sync + 'static> {
        #[cfg(not(windows))]
        {
            static SHARED_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
            Ok(SHARED_LOCK.lock().await)
        }
        #[cfg(windows)]
        {
            Ok(vrc_get_litedb::shared_mutex::SharedMutex::new(lock_name)
                .await?
                .lock_owned()
                .await?)
        }
    }
}

impl TokioIoTraitImpl for DefaultEnvironmentIo {
    #[inline]
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

    #[inline]
    pub fn location(&self) -> &Path {
        &self.root
    }
}

impl TokioIoTraitImpl for DefaultProjectIo {
    #[inline]
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

    async fn write_sync(&self, path: &Path, content: &[u8]) -> io::Result<()> {
        let path = self.resolve(path)?;
        let mut file = fs::File::create(&path).await?;
        file.write_all(content).await?;
        file.flush().await?;
        file.sync_data().await?;
        Ok(())
    }

    async fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(self.resolve(path)?).await
    }

    async fn remove_dir(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir(self.resolve(path)?).await
    }

    async fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir_all(self.resolve(path)?).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(self.resolve(from)?, self.resolve(to)?).await
    }

    async fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        fs::metadata(self.resolve(path)?).await.map(Into::into)
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
            .truncate(true)
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

impl FileStream for tokio_util::compat::Compat<fs::File> {}

pub type File = tokio_util::compat::Compat<fs::File>;

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

    async fn file_type(&self) -> io::Result<FileType> {
        self.inner.file_type().await.map(Into::into)
    }

    async fn metadata(&self) -> io::Result<Metadata> {
        self.inner.metadata().await.map(Into::into)
    }
}

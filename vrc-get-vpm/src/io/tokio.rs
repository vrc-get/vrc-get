use crate::io;
use crate::io::{
    EnvironmentIo, FileStream, FileSystemProjectIo, FileType, IoTrait, Metadata, ProjectIo,
};
use futures::{Stream, TryFutureExt};
use log::debug;
use std::ffi::OsString;
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

impl EnvironmentIo for DefaultEnvironmentIo {
    #[inline]
    fn resolve(&self, path: &Path) -> PathBuf {
        self.root.join(path)
    }

    #[cfg(feature = "vrc-get-litedb")]
    async fn connect_lite_db(&self) -> io::Result<crate::environment::VccDatabaseConnection> {
        use vrc_get_litedb::engine::{LiteEngine, LiteSettings};
        use vrc_get_litedb::tokio_fs::TokioStreamFactory;

        let path = EnvironmentIo::resolve(self, "vcc.liteDb".as_ref());
        let log_path = EnvironmentIo::resolve(self, "vcc.liteDb".as_ref());

        #[cfg(windows)]
        let lock = {
            use sha1::Digest;

            let path = path.to_string_lossy();
            let path_lower = path.to_lowercase();
            let mut sha1 = sha1::Sha1::new();
            sha1.update(path_lower.as_bytes());
            let hash = &sha1.finalize()[..];
            let hash_hex = hex::encode(hash);
            // this lock name is same as shared engine in litedb
            let name = format!("Global\\{hash_hex}.Mutex");

            Box::new(
                vrc_get_litedb::shared_mutex::SharedMutex::new(name)
                    .await?
                    .lock()
                    .await?,
            )
        };
        #[cfg(not(windows))]
        let lock = Box::new(());

        let engine = LiteEngine::new(LiteSettings {
            data_stream: Box::new(TokioStreamFactory::new(path.clone())),
            log_stream: Box::new(TokioStreamFactory::new(log_path.clone())),
            auto_build: false,
            collation: None,
        })
        .await?;

        Ok(crate::environment::VccDatabaseConnection::new(engine, lock))
    }

    #[cfg(feature = "experimental-project-management")]
    type ProjectIo = DefaultProjectIo;

    #[cfg(feature = "experimental-project-management")]
    fn new_project_io(&self, path: &Path) -> Self::ProjectIo {
        DefaultProjectIo::new(path.into())
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
}

impl ProjectIo for DefaultProjectIo {}

impl FileSystemProjectIo for DefaultProjectIo {
    #[inline]
    fn location(&self) -> &Path {
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

#[cfg(windows)]
mod win_mutex {
    use std::ffi::OsString;
    use std::io;
    use windows::Win32::Foundation::*;
    use windows::Win32::System::Threading::*;

    pub(super) struct MutexGuard {
        wait_sender: std::sync::mpsc::Sender<()>,
    }

    impl MutexGuard {
        pub async fn new(name: impl Into<OsString>) -> io::Result<Self> {
            let name = name.into();
            let (result_sender, mut result_receiver) =
                tokio::sync::mpsc::channel::<io::Result<()>>(1);
            let (wait_sender, wait_receiver) = std::sync::mpsc::channel::<()>();

            // create thread for mutex creation and free
            #[allow(unsafe_code)]
            std::thread::spawn(move || {
                // https://github.com/dotnet/runtime/blob/bcca12c1b14d25b3368106301748cb35c0894356/src/libraries/Common/src/Interop/Windows/Kernel32/Interop.Constants.cs#L12
                const MAXIMUM_ALLOWED: u32 = 0x02000000;
                const ACCESS_RIGHTS: u32 =
                    MAXIMUM_ALLOWED | PROCESS_SYNCHRONIZE.0 | MUTEX_MODIFY_STATE.0;

                let name = windows::core::HSTRING::from(&name);

                let handle = match unsafe { CreateMutexExW(None, &name, 0, ACCESS_RIGHTS) } {
                    Ok(handle) => handle,
                    Err(e) => {
                        // failed to create
                        result_sender.blocking_send(Err(e.into())).unwrap();
                        return;
                    }
                };

                unsafe {
                    let r = WaitForSingleObject(handle, INFINITE);
                    if r == WAIT_FAILED {
                        result_sender
                            .blocking_send(Err(io::Error::last_os_error()))
                            .unwrap();
                    }
                }

                result_sender.blocking_send(Ok(())).unwrap();

                wait_receiver.recv().ok();

                unsafe {
                    ReleaseMutex(handle).ok();
                    CloseHandle(handle).ok();
                }
            });

            result_receiver.recv().await.unwrap()?;

            Ok(Self { wait_sender })
        }
    }

    impl Drop for MutexGuard {
        fn drop(&mut self) {
            println!("unlock");
            self.wait_sender.send(()).unwrap();
        }
    }
}

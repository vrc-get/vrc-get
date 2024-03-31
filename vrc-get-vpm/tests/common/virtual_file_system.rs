use futures::Stream;
use indexmap::map::Entry;
use indexmap::IndexMap;
use std::ffi::{OsStr, OsString};
use std::future::Future;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::{error, io};
use vrc_get_vpm::io::{EnvironmentIo, ExitStatus, FileType, Metadata, ProjectIo};

pub(crate) use file_stream::*;

const NOTA_DIRECTORY: ErrorKind = ErrorKind::Other; // NotADirectory is unstable
const IS_DIRECTORY: ErrorKind = ErrorKind::Other; // IsADirectory is unstable

fn err<V, E>(kind: ErrorKind, error: E) -> io::Result<V>
where
    E: Into<Box<dyn error::Error + Send + Sync>>,
{
    Err(io::Error::new(kind, error))
}

/// The virtual file system is a TraitIo implementation for testing.
///
/// This struct implements All EnvironmentIo and ProjectIo methods.
pub struct VirtualFileSystem {
    root: DirectoryEntry,
}

impl VirtualFileSystem {
    pub fn new() -> Self {
        Self {
            root: DirectoryEntry::new(),
        }
    }

    pub async fn add_file(&self, path: &Path, content: &[u8]) -> io::Result<()> {
        let Some((dir_path, last)) = self.resolve2(path)? else {
            return err(IS_DIRECTORY, "is directory");
        };
        self.root
            .create_dir_all(&dir_path)
            .await?
            .create_file(last, true)
            .await?
            .set_content(content)
            .await;
        Ok(())
    }

    pub async fn deny_deletion(&self, path: &Path) -> io::Result<()> {
        let Some((dir_path, last)) = self.resolve2(path)? else {
            return err(IS_DIRECTORY, "is directory");
        };
        self.root
            .get_folder(&dir_path)
            .await?
            .get(last)
            .await?
            .as_file()?
            .deny_deletion();
        Ok(())
    }
}

impl VirtualFileSystem {
    fn resolve<'a>(&self, path: &'a Path) -> io::Result<Vec<&'a OsStr>> {
        let mut result = Vec::new();

        for x in path.components() {
            match x {
                Component::Prefix(_) | Component::RootDir => {
                    panic!("absolute path")
                }
                Component::CurDir => continue,
                Component::ParentDir => {
                    if result.pop().is_none() {
                        panic!("accessing parent folder")
                    }
                }
                Component::Normal(component) => result.push(component),
            }
        }

        Ok(result)
    }

    fn resolve2<'a>(&self, path: &'a Path) -> io::Result<Option<(Vec<&'a OsStr>, &'a OsStr)>> {
        let mut resolved = self.resolve(path)?;
        if resolved.is_empty() {
            Ok(None)
        } else {
            let last = resolved.remove(resolved.len() - 1);
            Ok(Some((resolved, last)))
        }
    }
}

impl vrc_get_vpm::io::IoTrait for VirtualFileSystem {
    async fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.root.create_dir_all(&self.resolve(path)?).await?;
        Ok(())
    }

    async fn write(&self, path: &Path, content: &[u8]) -> io::Result<()> {
        let Some((dir_path, last)) = self.resolve2(path)? else {
            return err(IS_DIRECTORY, "is directory");
        };
        let file = self
            .root
            .get_folder(&dir_path)
            .await?
            .create_file(last, false)
            .await?;
        file.set_content(content).await;
        Ok(())
    }

    async fn remove_file(&self, path: &Path) -> io::Result<()> {
        let Some((dir_path, last)) = self.resolve2(path)? else {
            return err(IS_DIRECTORY, "is directory");
        };
        self.root
            .get_folder(&dir_path)
            .await?
            .remove_file(last)
            .await?;
        Ok(())
    }

    async fn remove_dir(&self, path: &Path) -> io::Result<()> {
        let Some((dir_path, last)) = self.resolve2(path)? else {
            return err(ErrorKind::PermissionDenied, "removing root");
        };
        self.root
            .get_folder(&dir_path)
            .await?
            .remove_dir_all(last)
            .await?;
        Ok(())
    }

    async fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        let Some((dir_path, last)) = self.resolve2(path)? else {
            return err(ErrorKind::PermissionDenied, "removing root");
        };
        self.root
            .get_folder(&dir_path)
            .await?
            .remove_dir_all(last)
            .await?;
        Ok(())
    }

    async fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let Some((from_dir, from_last)) = self.resolve2(from)? else {
            return err(ErrorKind::PermissionDenied, "moving root");
        };
        let Some((to_dir, to_last)) = self.resolve2(to)? else {
            return err(ErrorKind::PermissionDenied, "moving to root");
        };

        let from_dir = self.root.get_folder(&from_dir).await?;
        let to_dir = self.root.get_folder(&to_dir).await?;

        let mut from_dir = from_dir.backed.lock().unwrap();
        let mut to_dir = to_dir.backed.lock().unwrap();

        let from_entry = match from_dir.entry(from_last.to_os_string()) {
            Entry::Occupied(e) => e,
            Entry::Vacant(_) => return err(ErrorKind::NotFound, "file not found"),
        };

        let to_entry = match to_dir.entry(to_last.to_os_string()) {
            Entry::Occupied(_) => return err(ErrorKind::AlreadyExists, "file exists"),
            Entry::Vacant(e) => e,
        };

        to_entry.insert(from_entry.shift_remove());

        Ok(())
    }

    async fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        let Some((dir_path, last)) = self.resolve2(path)? else {
            return Ok(Metadata::dir());
        };
        Ok(self
            .root
            .get_folder(&dir_path)
            .await?
            .get(last)
            .await?
            .metadata())
    }

    type DirEntry = DirEntry;
    type ReadDirStream = ReadDirStream;

    async fn read_dir(&self, path: &Path) -> io::Result<Self::ReadDirStream> {
        let root = self.root.get_folder(&self.resolve(path)?).await?;

        Ok(ReadDirStream::new(root))
    }

    type FileStream = FileStream;

    async fn create_new(&self, path: &Path) -> io::Result<Self::FileStream> {
        let Some((dir_path, last)) = self.resolve2(path)? else {
            return err(IS_DIRECTORY, "is directory");
        };

        Ok(FileStream::new(
            self.root
                .get_folder(&dir_path)
                .await?
                .create_file(last, true)
                .await?
                .content
                .clone(),
        ))
    }

    async fn create(&self, path: &Path) -> io::Result<Self::FileStream> {
        let Some((dir_path, last)) = self.resolve2(path)? else {
            return err(IS_DIRECTORY, "is directory");
        };

        Ok(FileStream::new(
            self.root
                .get_folder(&dir_path)
                .await?
                .create_file(last, false)
                .await?
                .content
                .clone(),
        ))
    }

    async fn open(&self, path: &Path) -> io::Result<Self::FileStream> {
        let Some((dir_path, last)) = self.resolve2(path)? else {
            return err(IS_DIRECTORY, "is directory");
        };

        Ok(FileStream::new(
            self.root
                .get_folder(&dir_path)
                .await?
                .get(last)
                .await?
                .into_file()?
                .content
                .clone(),
        ))
    }
}

impl EnvironmentIo for VirtualFileSystem {
    fn resolve(&self, path: &Path) -> PathBuf {
        self.resolve(path)
            .expect("unexpected full path")
            .iter()
            .collect()
    }

    #[cfg(feature = "vrc-get-litedb")]
    fn connect_lite_db(&self) -> io::Result<vrc_get_litedb::DatabaseConnection> {
        err(ErrorKind::Unsupported, "lite db")
    }

    #[cfg(feature = "experimental-project-management")]
    type ProjectIo = VirtualFileSystem;

    #[cfg(feature = "experimental-project-management")]
    fn new_project_io(&self, _: &Path) -> Self::ProjectIo {
        panic!("not implemented") // TODO: implement
    }
}

impl ProjectIo for VirtualFileSystem {}

#[derive(Clone)]
enum FileSystemEntry {
    File(FileEntry),
    Directory(DirectoryEntry),
}

impl FileSystemEntry {
    fn metadata(&self) -> Metadata {
        match self {
            FileSystemEntry::File(_) => Metadata::file(),
            FileSystemEntry::Directory(_) => Metadata::dir(),
        }
    }

    fn into_file(self) -> io::Result<FileEntry> {
        match self {
            FileSystemEntry::File(e) => Ok(e),
            FileSystemEntry::Directory(_) => err(IS_DIRECTORY, "is a directory"),
        }
    }

    fn into_directory(self) -> io::Result<DirectoryEntry> {
        match self {
            FileSystemEntry::File(_) => err(NOTA_DIRECTORY, "is a file"),
            FileSystemEntry::Directory(e) => Ok(e),
        }
    }

    fn as_file(&self) -> io::Result<&FileEntry> {
        match self {
            FileSystemEntry::File(e) => Ok(e),
            FileSystemEntry::Directory(_) => err(IS_DIRECTORY, "is a directory"),
        }
    }

    fn as_directory(&self) -> io::Result<&DirectoryEntry> {
        match self {
            FileSystemEntry::File(_) => err(NOTA_DIRECTORY, "is a file"),
            FileSystemEntry::Directory(e) => Ok(e),
        }
    }
}

#[derive(Clone)]
struct DirectoryEntry {
    backed: Arc<Mutex<IndexMap<OsString, FileSystemEntry>>>,
}

impl DirectoryEntry {
    fn new() -> Self {
        Self {
            backed: Arc::new(Mutex::new(IndexMap::new())),
        }
    }

    async fn get(&self, name: &OsStr) -> io::Result<FileSystemEntry> {
        self.backed
            .lock()
            .unwrap()
            .get(name)
            .map(Clone::clone)
            .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "file not found"))
    }

    async fn get_folder(&self, path: &[&OsStr]) -> io::Result<DirectoryEntry> {
        let mut current = self.clone();

        for component in path {
            current = current.get(component).await?.into_directory()?;
        }

        Ok(current)
    }

    async fn create_dir_all(&self, path: &[&OsStr]) -> io::Result<DirectoryEntry> {
        let mut current = self.clone();

        for component in path {
            let mut locked = current.backed.lock().unwrap();
            let next = match locked.entry(component.to_os_string()) {
                Entry::Occupied(e) => e.into_mut().as_directory()?.clone(),
                Entry::Vacant(e) => e
                    .insert(FileSystemEntry::Directory(DirectoryEntry::new()))
                    .as_directory()
                    .unwrap()
                    .clone(),
            };
            drop(locked);
            current = next;
        }

        Ok(current)
    }

    async fn create_file(&self, name: &OsStr, new: bool) -> io::Result<FileEntry> {
        let mut backed = self.backed.lock().unwrap();
        match backed.entry(name.to_os_string()) {
            Entry::Occupied(e) => match e.into_mut() {
                FileSystemEntry::File(_) if new => err(ErrorKind::AlreadyExists, "file exists"),
                FileSystemEntry::File(entry) => Ok(entry.clone()),
                FileSystemEntry::Directory(_) => err(IS_DIRECTORY, "directory exists"),
            },
            Entry::Vacant(e) => {
                let FileSystemEntry::File(e) = e.insert(FileSystemEntry::File(FileEntry::new()))
                else {
                    unreachable!()
                };
                Ok(e.clone())
            }
        }
    }

    async fn remove_file(&self, name: &OsStr) -> io::Result<FileEntry> {
        let mut backed = self.backed.lock().unwrap();
        match backed.entry(name.to_os_string()) {
            Entry::Occupied(mut e) => {
                let file = e.get_mut().as_file()?;
                if !file.can_remove() {
                    return err(ErrorKind::PermissionDenied, "file is locked");
                }
                Ok(e.shift_remove().into_file().unwrap())
            }
            Entry::Vacant(_) => err(ErrorKind::NotFound, "file not found"),
        }
    }

    async fn remove_dir(&self, name: &OsStr) -> io::Result<DirectoryEntry> {
        let mut backed = self.backed.lock().unwrap();
        match backed.entry(name.to_os_string()) {
            Entry::Occupied(mut e) => {
                let as_dir = e.get_mut().as_directory()?;
                if !as_dir.backed.lock().unwrap().is_empty() {
                    return err(ErrorKind::Other, "directory not empty"); // DirectoryNotEmpty is unstable
                }
                Ok(e.shift_remove().into_directory().unwrap())
            }
            Entry::Vacant(_) => err(ErrorKind::NotFound, "file not found"),
        }
    }

    async fn remove_dir_all(&self, name: &OsStr) -> io::Result<DirectoryEntry> {
        let mut backed = self.backed.lock().unwrap();
        match backed.entry(name.to_os_string()) {
            Entry::Occupied(mut e) => {
                e.get_mut().as_directory()?.clear()?;
                Ok(e.shift_remove().into_directory().unwrap())
            }
            Entry::Vacant(_) => err(ErrorKind::NotFound, "file not found"),
        }
    }

    fn clear(&self) -> io::Result<()> {
        let mut backed = self.backed.lock().unwrap();
        while let Some((_, child)) = backed.last() {
            match child {
                FileSystemEntry::File(f) => {
                    if !f.can_remove() {
                        return err(ErrorKind::PermissionDenied, "file is locked");
                    }
                }
                FileSystemEntry::Directory(d) => d.clear()?,
            }
            let last = backed.len() - 1;
            backed.swap_remove_index(last);
        }
        Ok(())
    }
}

#[derive(Clone)]
struct FileEntry {
    content: Arc<Mutex<FileContent>>,
}

struct FileContent {
    content: Vec<u8>,
    locked: bool,
}

impl FileContent {
    fn new() -> Self {
        Self {
            content: Vec::new(),
            locked: false,
        }
    }
}

impl FileEntry {
    fn new() -> Self {
        Self {
            content: Arc::new(Mutex::new(FileContent::new())),
        }
    }

    pub(crate) async fn set_content(&self, content: &[u8]) {
        self.content.lock().unwrap().content = content.to_vec();
    }

    pub(crate) fn deny_deletion(&self) {
        self.content.lock().unwrap().locked = true;
    }

    pub(crate) fn can_remove(&self) -> bool {
        !self.content.lock().unwrap().locked
    }
}

pub struct ReadDirStream {
    index: usize,
    dir: DirectoryEntry,
}

impl ReadDirStream {
    fn new(dir: DirectoryEntry) -> Self {
        Self { index: 0, dir }
    }
}

impl Stream for ReadDirStream {
    type Item = io::Result<DirEntry>;

    fn poll_next(mut self: Pin<&mut Self>, _: &mut Context) -> Poll<Option<Self::Item>> {
        let locked = self.dir.backed.lock().unwrap();
        let Some((name, entry)) = locked.get_index(self.index) else {
            return Poll::Ready(None);
        };
        let entry = DirEntry::new(name, entry.metadata());
        drop(locked);
        self.index += 1;
        Poll::Ready(Some(Ok(entry)))
    }
}

pub struct DirEntry {
    name: OsString,
    metadata: Metadata,
}

impl DirEntry {
    fn new(name: &OsStr, metadata: Metadata) -> Self {
        Self {
            name: name.to_os_string(),
            metadata,
        }
    }
}

impl vrc_get_vpm::io::DirEntry for DirEntry {
    fn file_name(&self) -> OsString {
        self.name.clone()
    }

    async fn file_type(&self) -> io::Result<FileType> {
        Ok(self.metadata.file_type())
    }

    async fn metadata(&self) -> io::Result<Metadata> {
        Ok(self.metadata.clone())
    }
}

mod file_stream {
    use crate::common::virtual_file_system::FileContent;
    use futures::{AsyncRead, AsyncSeek, AsyncWrite};
    use std::io;
    use std::io::{ErrorKind, SeekFrom};
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll};

    pub struct FileStream {
        content: Arc<Mutex<FileContent>>,
        position: usize,
    }

    impl FileStream {
        pub(super) fn new(content: Arc<Mutex<FileContent>>) -> Self {
            Self {
                content,
                position: 0,
            }
        }
    }

    impl AsyncSeek for FileStream {
        fn poll_seek(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
            pos: SeekFrom,
        ) -> Poll<io::Result<u64>> {
            match pos {
                SeekFrom::Start(position) => {
                    self.position = position
                        .try_into()
                        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "invalid position"))?;
                    Poll::Ready(Ok(self.position as u64))
                }
                SeekFrom::Current(offset) => {
                    let offset = offset
                        .try_into()
                        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "invalid position"))?;
                    self.position = self.position.checked_add_signed(offset).ok_or_else(|| {
                        io::Error::new(ErrorKind::InvalidInput, "invalid position")
                    })?;
                    Poll::Ready(Ok(self.position as u64))
                }
                SeekFrom::End(offset) => {
                    let offset = offset
                        .try_into()
                        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "invalid position"))?;

                    let lock = self.content.clone();
                    let guard = lock.lock().unwrap();
                    self.position =
                        guard
                            .content
                            .len()
                            .checked_add_signed(offset)
                            .ok_or_else(|| {
                                io::Error::new(ErrorKind::InvalidInput, "invalid position")
                            })?;
                    Poll::Ready(Ok(self.position as u64))
                }
            }
        }
    }

    impl AsyncRead for FileStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            let lock = self.content.clone();
            let guard = lock.lock().unwrap();
            let len = guard.content.len();
            let remaining = len - self.position;
            let to_copy = buf.len().min(remaining);
            buf[..to_copy].copy_from_slice(&guard.content[self.position..][..to_copy]);
            self.position += to_copy;
            Poll::Ready(Ok(to_copy))
        }
    }

    impl AsyncWrite for FileStream {
        fn poll_write(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            let lock = self.content.clone();
            let mut guard = lock.lock().unwrap();
            let new_len = self.position + buf.len();
            if new_len > guard.content.len() {
                guard.content.resize(new_len, 0);
            }
            guard.content[self.position..][..buf.len()].copy_from_slice(buf);

            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }
}

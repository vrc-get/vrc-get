use crate::state::*;

use futures::future::try_join_all;
use futures::{AsyncRead, AsyncReadExt};
use log::warn;
use stable_deref_trait::StableDeref;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::future::Future;
use std::io;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, ready};
use tar::Header;
use tokio::sync::Semaphore;
use vrc_get_vpm::UnityProject;
use vrc_get_vpm::io::DefaultProjectIo;
use yoke::{CloneableCart, Yoke, Yokeable};

pub(crate) fn home_dir() -> PathBuf {
    dirs_next::home_dir().expect("Failed to get home directory")
}

pub(crate) fn default_backup_path() -> String {
    let mut home = home_dir();
    home.extend(&["ALCOM", "Backups"]);
    home.to_string_lossy().into_owned()
}

pub(crate) fn project_backup_path<'env>(settings: &'env mut SettingMutRef<'_>) -> &'env str {
    if settings.project_backup_path().is_none() {
        settings.set_project_backup_path(&default_backup_path());
        settings.require_save();
    }

    settings.project_backup_path().unwrap()
}

pub(crate) fn default_default_project_path() -> String {
    let mut home = home_dir();
    home.extend(&["ALCOM", "Projects"]);
    home.to_string_lossy().into_owned()
}

pub(crate) fn default_project_path<'env>(settings: &'env mut SettingMutRef<'_>) -> &'env str {
    if settings.default_project_path().is_none() {
        settings.set_default_project_path(&default_default_project_path());
        settings.require_save();
    }

    settings.default_project_path().unwrap()
}

pub(crate) fn find_existing_parent_dir(path: &Path) -> Option<&Path> {
    let mut parent = path;
    loop {
        if std::fs::metadata(parent)
            .map(|x| x.is_dir())
            .unwrap_or(false)
        {
            return Some(parent);
        }

        match parent.parent() {
            Some(p) => parent = p,
            None => return None,
        }
    }
}

pub(crate) fn find_existing_parent_dir_or_home(path: &Path) -> Cow<'_, Path> {
    find_existing_parent_dir(path)
        .map(Cow::Borrowed)
        .unwrap_or_else(|| Cow::Owned(home_dir()))
}

pub(crate) trait YokeExt<Y: for<'a> Yokeable<'a>, C> {
    fn try_map_project_async<'this, P, F, E, Fut>(
        &'this self,
        f: F,
    ) -> impl Future<Output = Result<Yoke<P, C>, E>>
    where
        P: for<'a> Yokeable<'a>,
        C: CloneableCart + StableDeref,
        Fut: Future<Output = Result<<P as Yokeable<'this>>::Output, E>>,
        <C as Deref>::Target: 'this,
        F: FnOnce(
            &'this <C as Deref>::Target,
            &'this <Y as Yokeable<'this>>::Output,
            PhantomData<&'this ()>,
        ) -> Fut;
}

impl<Y: for<'a> Yokeable<'a>, C> YokeExt<Y, C> for Yoke<Y, C> {
    /// ```rust,compile_fail
    /// # async fn test<Y: for<'a> Yokeable<'a>, C: CloneableCart + StableDeref>(yoke: Yoke<Y, C>) {
    /// let mut outer_arg = None;
    /// yoke.try_map_project_async::<u8, _, (), _>(|_, yokable, _| async move {
    ///     outer_arg = Some(yokable);
    ///     Ok(0)
    /// })
    /// .await;
    /// drop(yoke);
    /// outer_arg.unwrap(); // Errors!
    /// # }
    /// ```
    async fn try_map_project_async<'this, P, F, E, Fut>(&'this self, f: F) -> Result<Yoke<P, C>, E>
    where
        P: for<'a> Yokeable<'a>,
        C: CloneableCart + StableDeref,
        Fut: Future<Output = Result<<P as Yokeable<'this>>::Output, E>>,
        F: FnOnce(
            &'this <C as Deref>::Target,
            &'this <Y as Yokeable<'this>>::Output,
            PhantomData<&'this ()>,
        ) -> Fut,
    {
        let data = f(self.backing_cart(), self.get(), PhantomData).await?;

        unsafe {
            Ok(
                Yoke::new_always_owned(P::make(data))
                    .replace_cart(|()| self.backing_cart().clone()),
            )
        }
    }
}

#[derive(Debug)]
pub struct FileSystemTree {
    relative_path: String,
    absolute_path: PathBuf,
    children: Vec<FileSystemTree>,
}

impl FileSystemTree {
    fn new_file(relative_path: String, absolute_path: PathBuf) -> Self {
        assert!(!relative_path.is_empty() && !relative_path.ends_with('/'));
        Self {
            relative_path,
            absolute_path,
            children: Vec::new(),
        }
    }

    fn new_dir(
        relative_path: String,
        absolute_path: PathBuf,
        children: Vec<FileSystemTree>,
    ) -> Self {
        assert!(relative_path.is_empty() || relative_path.ends_with('/'));
        Self {
            relative_path,
            absolute_path,
            children,
        }
    }

    pub fn is_dir(&self) -> bool {
        self.relative_path.is_empty() || self.relative_path.ends_with('/')
    }

    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub fn absolute_path(&self) -> &Path {
        &self.absolute_path
    }

    pub fn recursive(&self) -> FileSystemTreeRecursive<'_> {
        FileSystemTreeRecursive {
            stack: vec![(self, 0)],
        }
    }

    #[allow(dead_code)]
    pub fn iter(&self) -> FileSystemTreeIter<'_> {
        FileSystemTreeIter {
            back: self.children.iter(),
        }
    }

    /// Count all files and directories in the tree excluding the root
    pub fn count_all(&self) -> usize {
        self.recursive().count()
    }
}

pub struct FileSystemTreeRecursive<'a> {
    stack: Vec<(&'a FileSystemTree, usize)>,
}

impl<'a> Iterator for FileSystemTreeRecursive<'a> {
    type Item = &'a FileSystemTree;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (tree, index) = self.stack.pop()?;

            if index < tree.children.len() {
                self.stack.push((tree, index + 1));
                let new_ent = &tree.children[index];
                self.stack.push((new_ent, 0));
                return Some(new_ent);
            }
        }
    }
}

pub struct FileSystemTreeIter<'a> {
    back: std::slice::Iter<'a, FileSystemTree>,
}

impl<'a> Iterator for FileSystemTreeIter<'a> {
    type Item = &'a FileSystemTree;

    fn next(&mut self) -> Option<Self::Item> {
        self.back.next()
    }
}

pub async fn collect_notable_project_files_tree(
    path_buf: PathBuf,
    exclude_vpm: bool,
    backup: bool,
) -> io::Result<FileSystemTree> {
    // relative path must end with '/' or empty
    async fn read_dir_to_tree(
        relative: String,
        absolute: PathBuf,
        excluded_packages: &[String],
        backup: bool,
        semaphore: Arc<Semaphore>,
    ) -> io::Result<FileSystemTree> {
        // we have semaphore to limit simultaneous file access.
        let semaphore_scope = semaphore.acquire().await.unwrap();
        let mut read_dir = tokio::fs::read_dir(&absolute).await?;

        // relative, entry, is_dir
        let mut entries = Vec::new();

        while let Some(entry) = read_dir.next_entry().await? {
            let Ok(file_name) = entry.file_name().into_string() else {
                // non-utf8 file name
                warn!("skipping non-utf8 file name: {}", entry.path().display());
                continue;
            };
            log::trace!("process: {relative}{file_name}");

            let new_relative;
            let is_dir;

            let file_type = entry.file_type().await?;

            if file_type.is_symlink() {
                // skip symlink
                // TODO: further handling
                warn!("skipping symlink: {}", entry.path().display());
                continue;
            }

            if entry.file_type().await?.is_dir() {
                let lower_name = file_name.to_ascii_lowercase();
                if relative.is_empty() {
                    match lower_name.as_str() {
                        "logs" | "obj" | "temp" => {
                            continue;
                        }
                        _ => {}
                    }
                } else if relative.eq_ignore_ascii_case("packages/") {
                    // the package is excluded
                    if excluded_packages.contains(&lower_name) {
                        continue;
                    }
                }

                if backup {
                    // for backups, we exclude the library directory
                    if relative.starts_with_ascii_ignore("library") {
                        // some people use multiple library folder to speed up switching platforms,
                        // so we use starts_with way for matching

                        // It's inside a library directory, all directories will be ignored

                        // We won't skip Library directory directly
                        // since we want to keep LastSceneManagerSetup.txt as a exception
                        continue;
                    }
                }

                if lower_name.as_str() == ".git" {
                    // any .git folder should be ignored
                    continue;
                }

                new_relative = format!("{relative}{file_name}/");
                is_dir = true;
            } else {
                if backup {
                    // for backups, we exclude the library directory
                    if relative.starts_with_ascii_ignore("library") {
                        // some people use multiple library folder to speed up switching platforms,
                        // so we use starts_with way for matching

                        // It's inside a library directory, all files except for few files

                        if file_name.eq_ignore_ascii_case("LastSceneManagerSetup.txt") {
                            // `LastSceneManagerSetup.txt` will preserve the information which
                            // scene was opened last time.
                            //
                            // Many avatar project users doesn't understand they're editing scene,
                            // and they don't understand they can create another new scene,
                            // and can be opened from project window.
                            // Therefore, some user says that "I restored from backup, but avatars are
                            // go away from my project" even though they're opening another scene.
                            // Therefore, we decided to keep this file where possible.
                        } else {
                            continue;
                        }
                    }
                }
                new_relative = format!("{relative}{file_name}");
                is_dir = false;
            }

            entries.push((new_relative, entry, is_dir));
        }

        // release semaphore since directory traversal has finished.
        drop(semaphore_scope);

        let children = try_join_all(entries.into_iter().map({
            |(relative, entry, is_dir)| {
                let semaphore = semaphore.clone();
                async move {
                    if is_dir {
                        read_dir_to_tree(
                            relative,
                            entry.path(),
                            excluded_packages,
                            backup,
                            semaphore,
                        )
                        .await
                    } else {
                        Ok(FileSystemTree::new_file(relative, entry.path()))
                    }
                }
            }
        }))
        .await?;

        Ok(FileSystemTree::new_dir(relative, absolute, children))
    }

    let excluded_packages = if exclude_vpm {
        async fn get_packages(path: &Path) -> Option<Vec<String>> {
            let unity_project = UnityProject::load(DefaultProjectIo::new(path.into()))
                .await
                .ok()?;
            Some(
                unity_project
                    .locked_packages()
                    .map(|x| x.name().into())
                    .collect(),
            )
        }
        get_packages(&path_buf).await.unwrap_or_default()
    } else {
        vec![]
    };

    let semaphore = Arc::new(Semaphore::new(100));

    read_dir_to_tree(
        String::new(),
        path_buf,
        &excluded_packages,
        backup,
        semaphore,
    )
    .await
}

pub trait PathExt {
    fn with_added_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf;
}

impl PathExt for PathBuf {
    fn with_added_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf {
        let mut new_path = self.clone();
        #[allow(unstable_name_collisions)]
        new_path.add_extension(extension);
        new_path
    }
}

pub trait PathBufExt {
    fn add_extension<S: AsRef<OsStr>>(&mut self, extension: S) -> bool;
}

impl PathBufExt for PathBuf {
    fn add_extension<S: AsRef<OsStr>>(&mut self, extension: S) -> bool {
        fn _add_extension(this: &mut PathBuf, extension: &OsStr) -> bool {
            if this.file_name().is_none() {
                return false;
            }

            if let Some(ext) = this.extension() {
                let mut new_ext = ext.to_os_string();
                new_ext.push(".");
                new_ext.push(extension);
                this.set_extension(new_ext);
            } else {
                this.set_extension(extension);
            }
            true
        }

        _add_extension(self, extension.as_ref())
    }
}

pub trait StrExt {
    fn starts_with_ascii_ignore(&self, pat: &str) -> bool;
}

impl StrExt for str {
    fn starts_with_ascii_ignore(&self, pat: &str) -> bool {
        let Some(heading) = self.get(..pat.len()) else {
            return false;
        };
        heading.eq_ignore_ascii_case(pat)
    }
}

pub struct TarArchive<R: ?Sized + AsyncRead + Unpin> {
    to_skip: u64,
    reader: R,
}

pub struct TarEntry<'a, R: AsyncRead + Unpin> {
    archive: &'a mut TarArchive<R>,
    remaining: u64,
    header: Header,
}

impl<R: AsyncRead + Unpin> TarArchive<R> {
    pub fn new(reader: R) -> Self
    where
        R: Sized,
    {
        Self { reader, to_skip: 0 }
    }

    pub async fn next_entry(&mut self) -> io::Result<Option<TarEntry<'_, R>>> {
        const BLOCK_SIZE: u64 = 512;
        let mut header = Header::new_old();
        // skip bytes
        while self.to_skip != 0 {
            let size = std::cmp::min(self.to_skip, BLOCK_SIZE) as usize;
            self.reader
                .read_exact(&mut header.as_mut_bytes()[..size])
                .await?;
            self.to_skip -= size as u64;
        }
        self.reader.read_exact(header.as_mut_bytes()).await?;
        if header.as_bytes().iter().all(|&b| b == 0) {
            // the header is all zeros; trailing header
            return Ok(None);
        }

        // Make sure the checksum is ok
        let sum = (header.as_bytes()[..148].iter())
            .chain(&header.as_bytes()[156..])
            .fold(0, |a, b| a + (*b as u32))
            + 8 * b' ' as u32;
        let cksum = header.cksum()?;
        if sum != cksum {
            return Err(io::Error::other("archive header checksum mismatch"));
        }

        let size = header.size()?;
        let to_skip = (size + BLOCK_SIZE - 1) & !(BLOCK_SIZE - 1);
        self.to_skip = to_skip;
        Ok(Some(TarEntry {
            archive: self,
            remaining: size,
            header,
        }))
    }
}

impl<R: AsyncRead + Unpin> TarEntry<'_, R> {
    pub fn header(&self) -> &Header {
        &self.header
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for TarEntry<'_, R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let buffer_size = std::cmp::min(buf.len() as u64, self.remaining) as usize;
        let size =
            ready!(Pin::new(&mut self.archive.reader).poll_read(cx, &mut buf[..buffer_size])?);
        self.remaining -= size as u64;
        self.archive.to_skip -= size as u64;
        Poll::Ready(Ok(size))
    }
}

pub async fn trash_delete(path: PathBuf) -> Result<(), trash::Error> {
    tokio::runtime::Handle::current()
        .spawn_blocking(move || trash::delete(path))
        .await
        .unwrap()
}

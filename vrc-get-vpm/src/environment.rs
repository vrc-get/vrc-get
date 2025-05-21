mod repo_holder;
mod repo_source;
mod uesr_package_collection;
mod vpm_settings;
mod vrc_get_settings;

#[cfg(feature = "vrc-get-litedb")]
mod litedb;
mod package_collection;
mod package_installer;
#[cfg(feature = "experimental-project-management")]
mod project_management;
mod settings;
#[cfg(feature = "experimental-unity-management")]
mod unity_management;

use crate::io;
use crate::repository::RemoteRepository;
use crate::repository::local::LocalCachedRepository;
use crate::traits::HttpClient;
use crate::utils::to_vec_pretty_os_eol;
use futures::prelude::*;
use indexmap::IndexMap;
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::Path;
use url::Url;

use crate::io::{DefaultEnvironmentIo, DirEntry, IoTrait};
#[cfg(feature = "experimental-project-management")]
pub use project_management::*;
pub(crate) use repo_holder::RepoHolder;
pub(crate) use repo_source::RepoSource;
#[cfg(feature = "experimental-unity-management")]
pub use unity_management::*;

#[cfg(feature = "vrc-get-litedb")]
pub use litedb::VccDatabaseConnection;
pub use package_collection::PackageCollection;
pub use package_installer::PackageInstaller;
pub use settings::Settings;
pub use uesr_package_collection::UserPackageCollection;

const OFFICIAL_URL_STR: &str = "https://packages.vrchat.com/official?download";
const LOCAL_OFFICIAL_PATH: &str = "Repos/vrc-official.json";
const CURATED_URL_STR: &str = "https://packages.vrchat.com/curated?download";
const LOCAL_CURATED_PATH: &str = "Repos/vrc-curated.json";
const REPO_CACHE_FOLDER: &str = "Repos";

pub async fn add_remote_repo(
    settings: &mut Settings,
    url: Url,
    name: Option<&str>,
    headers: IndexMap<Box<str>, Box<str>>,
    io: &DefaultEnvironmentIo,
    http: &impl HttpClient,
) -> Result<(), AddRepositoryErr> {
    let (remote_repo, etag) = RemoteRepository::download(http, &url, &headers).await?;

    if !settings.can_add_remote_repo(&url, &remote_repo) {
        return Err(AddRepositoryErr::AlreadyAdded);
    }

    let mut local_cache = LocalCachedRepository::new(remote_repo, headers.clone());
    if let Some(etag) = etag {
        local_cache
            .vrc_get
            .get_or_insert_with(Default::default)
            .etag = etag;
    }

    io.create_dir_all(REPO_CACHE_FOLDER.as_ref()).await?;
    let file_name = write_new_repo(&local_cache, io).await?;
    let repo_path = io.resolve(format!("{}/{}", REPO_CACHE_FOLDER, file_name).as_ref());

    assert!(
        settings.add_remote_repo(&url, name, headers, local_cache.repo(), &repo_path),
        "add_remote_repo failed unexpectedly"
    );

    Ok(())
}

pub async fn cleanup_repos_folder(
    settings: &Settings,
    io: &DefaultEnvironmentIo,
) -> io::Result<()> {
    let mut uesr_repo_file_names = HashSet::<OsString>::from_iter([
        OsString::from("vrc-official.json"),
        OsString::from("vrc-curated.json"),
        // package cache management file used by VCC but not used by vrc-get
        OsString::from("package-cache.json"),
    ]);
    let repos_base = io.resolve(REPO_CACHE_FOLDER.as_ref());

    for x in settings.get_user_repos() {
        if let Ok(relative) = x.local_path().strip_prefix(&repos_base) {
            if let Some(file_name) = relative.file_name() {
                if relative
                    .parent()
                    .map(|x| x.as_os_str().is_empty())
                    .unwrap_or(true)
                {
                    // the file must be in direct child of
                    uesr_repo_file_names.insert(file_name.to_owned());
                }
            }
        }
    }

    let mut entry = io.read_dir(REPO_CACHE_FOLDER.as_ref()).await?;
    while let Some(entry) = entry.try_next().await? {
        let file_name: OsString = entry.file_name();
        if file_name.as_encoded_bytes().ends_with(b".json")
            && !uesr_repo_file_names.contains(&file_name)
            && entry.metadata().await.map(|x| x.is_file()).unwrap_or(false)
        {
            let mut path = OsString::with_capacity(REPO_CACHE_FOLDER.len() + 1 + file_name.len());
            path.push(REPO_CACHE_FOLDER);
            path.push(OsStr::new("/"));
            path.push(file_name);
            io.remove_file(path.as_ref()).await?;
        }
    }

    Ok(())
}

async fn write_new_repo(
    local_cache: &LocalCachedRepository,
    io: &DefaultEnvironmentIo,
) -> io::Result<String> {
    io.create_dir_all(REPO_CACHE_FOLDER.as_ref()).await?;

    // [0-9a-zA-Z._-]+
    fn is_id_name_for_file(id: &str) -> bool {
        !id.is_empty()
            && id
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'.' | b'_' | b'-'))
    }

    // try id.json
    let id_names = local_cache
        .id()
        .filter(|id| is_id_name_for_file(id))
        .map(|id| format!("{}.json", id))
        .into_iter();

    // finally generate with uuid v4.
    // note: this iterator is endless. Consumes uuidv4 infinitely.
    let guid_names = std::iter::from_fn(|| Some(format!("{}.json", uuid::Uuid::new_v4())));

    for file_name in id_names.chain(guid_names) {
        match io
            .create_new(format!("{}/{}", REPO_CACHE_FOLDER, file_name).as_ref())
            .await
        {
            Ok(mut file) => {
                file.write_all(&to_vec_pretty_os_eol(&local_cache)?).await?;
                file.flush().await?;

                return Ok(file_name);
            }
            Err(ref e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }

    unreachable!();
}

pub async fn clear_package_cache(io: &DefaultEnvironmentIo) -> io::Result<()> {
    let repo_folder_stream = io.read_dir(REPO_CACHE_FOLDER.as_ref()).await?;

    let pkg_folder_entries = repo_folder_stream.try_filter_map(|pkg_entry| async move {
        if pkg_entry.file_type().await?.is_dir() {
            return Ok(Some(pkg_entry));
        }
        Ok(None)
    });

    pkg_folder_entries
        .try_for_each_concurrent(None, |pkg_folder_entry| async move {
            let pkg_name = pkg_folder_entry.file_name();

            let pkg_folder_stream = io
                .read_dir(&Path::new(REPO_CACHE_FOLDER).join(pkg_folder_entry.file_name()))
                .await?
                .map_ok(move |inner| (pkg_name.clone(), inner));

            let cache_file_entries =
                pkg_folder_stream.try_filter_map(|(pkg_id, cache_entry)| async move {
                    let name = cache_entry.file_name();
                    let name = name.as_encoded_bytes();
                    if name.starts_with(b"vrc-get-")
                        && (name.ends_with(b".zip") || name.ends_with(b".zip.sha256"))
                        && cache_entry.file_type().await?.is_file()
                    {
                        return Ok(Some((pkg_id, cache_entry)));
                    }
                    Ok(None)
                });

            cache_file_entries
                .try_for_each_concurrent(None, |(pkg_id, cache_entry)| async move {
                    let file_path = Path::new(REPO_CACHE_FOLDER)
                        .join(pkg_id)
                        .join(cache_entry.file_name());
                    io.remove_file(&file_path).await?;
                    Ok(())
                })
                .await?;

            Ok(())
        })
        .await?;

    Ok(())
}

pub enum AddUserPackageResult {
    Success,
    NonAbsolute,
    BadPackage,
    AlreadyAdded,
}

#[derive(Debug)]
pub enum AddRepositoryErr {
    Io(io::Error),
    AlreadyAdded,
    OfflineMode,
}

impl fmt::Display for AddRepositoryErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddRepositoryErr::Io(ioerr) => fmt::Display::fmt(ioerr, f),
            AddRepositoryErr::AlreadyAdded => f.write_str("already repository added"),
            AddRepositoryErr::OfflineMode => {
                f.write_str("you can't add remote repo in offline mode")
            }
        }
    }
}

impl std::error::Error for AddRepositoryErr {}

impl From<io::Error> for AddRepositoryErr {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for AddRepositoryErr {
    fn from(value: serde_json::Error) -> Self {
        Self::Io(value.into())
    }
}

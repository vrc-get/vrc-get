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
use crate::repository::local::LocalCachedRepository;
use crate::repository::RemoteRepository;
use crate::structs::setting::UserRepoSetting;
use crate::traits::HttpClient;
use crate::traits::PackageCollection as _;
use crate::utils::to_vec_pretty_os_eol;
use crate::{PackageInfo, PackageManifest, VersionSelector};
use futures::future::join_all;
use futures::prelude::*;
use indexmap::IndexMap;
use itertools::Itertools;
use log::error;
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fmt::Write;
use std::fs::remove_file;
use std::path::{Path, PathBuf};
use url::Url;

use crate::io::{DirEntry, EnvironmentIo};
#[cfg(feature = "experimental-project-management")]
pub use project_management::*;
pub(crate) use repo_holder::RepoHolder;
pub(crate) use repo_source::RepoSource;

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

/// This struct holds global state (will be saved on %LOCALAPPDATA% of VPM.
#[derive(Debug)]
pub struct Environment {
    collection: PackageCollection,
    settings: Settings,
}

impl Environment {
    pub async fn load(io: &impl EnvironmentIo) -> io::Result<Self> {
        Ok(Self {
            collection: PackageCollection {
                repositories: HashMap::new(),
                user_packages: Vec::new(),
            },
            settings: Settings::load(io).await?,
        })
    }

    /// Reload configuration files on the filesystem.
    /// This doesn't update repository cache or user package cache.
    /// Please call [`load_package_infos`] after this method.
    ///
    /// [`load_package_infos`]: Environment::load_package_infos
    pub async fn reload(&mut self, io: &impl EnvironmentIo) -> io::Result<()> {
        self.settings = Settings::load(io).await?;
        Ok(())
    }

    pub async fn save(&mut self, io: &impl EnvironmentIo) -> io::Result<()> {
        self.settings.save(io).await?;
        Ok(())
    }
}

impl Environment {
    pub async fn load_package_infos(
        &mut self,
        io: &impl EnvironmentIo,
        http: Option<&impl HttpClient>,
    ) -> io::Result<()> {
        self.collection = PackageCollection::load(&self.settings, io, http).await?;
        self.settings.update_id(&self.collection);
        let duplicated_repos = self.settings.remove_id_duplication();
        self.collection
            .remove_repositories(&duplicated_repos, io)
            .await;

        Ok(())
    }

    pub async fn load_user_package_infos(&mut self, io: &impl EnvironmentIo) -> io::Result<()> {
        let user_packages = UserPackageCollection::load(&self.settings, io).await;

        self.collection.user_packages = user_packages.into_packages();

        Ok(())
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn new_package_collection(&self) -> PackageCollection {
        self.collection.clone()
    }
}

impl Environment {
    pub fn get_repos(&self) -> impl Iterator<Item = &'_ LocalCachedRepository> {
        self.collection.repositories.values()
    }

    pub fn find_whole_all_packages(
        &self,
        version_selector: VersionSelector,
        filter: impl Fn(&PackageManifest) -> bool,
    ) -> Vec<PackageInfo> {
        self.collection
            .get_all_packages()
            .filter(|x| version_selector.satisfies(x.package_json()))
            .into_group_map_by(|x| x.name())
            .values()
            .map(|versions| versions.iter().max_by_key(|x| x.version()).unwrap())
            .filter(|x| filter(x.package_json()))
            .copied()
            .collect()
    }

    pub fn get_user_repos(&self) -> &[UserRepoSetting] {
        self.settings.get_user_repos()
    }

    pub async fn add_remote_repo(
        &mut self,
        url: Url,
        name: Option<&str>,
        headers: IndexMap<Box<str>, Box<str>>,
        io: &impl EnvironmentIo,
        http: &impl HttpClient,
    ) -> Result<(), AddRepositoryErr> {
        let (remote_repo, etag) = RemoteRepository::download(http, &url, &headers).await?;

        if !self.settings.can_add_remote_repo(&url, &remote_repo) {
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
        let file_name = self.write_new_repo(&local_cache, io).await?;
        let repo_path = io.resolve(format!("{}/{}", REPO_CACHE_FOLDER, file_name).as_ref());

        assert!(
            self.settings
                .add_remote_repo(&url, name, headers, local_cache.repo(), &repo_path),
            "add_remote_repo failed unexpectedly"
        );

        Ok(())
    }

    async fn write_new_repo(
        &self,
        local_cache: &LocalCachedRepository,
        io: &impl EnvironmentIo,
    ) -> io::Result<String> {
        io.create_dir_all(REPO_CACHE_FOLDER.as_ref()).await?;

        // [0-9a-zA-Z._-]+
        fn is_id_name_for_file(id: &str) -> bool {
            !id.is_empty()
                && id.bytes().all(
                    |b| matches!(b, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'.' | b'_' | b'-'),
                )
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

    pub fn add_local_repo(
        &mut self,
        path: &Path,
        name: Option<&str>,
    ) -> Result<(), AddRepositoryErr> {
        if self.settings.add_local_repo(path, name) {
            Ok(())
        } else {
            Err(AddRepositoryErr::AlreadyAdded)
        }
    }

    pub async fn remove_repo(&mut self, condition: impl Fn(&UserRepoSetting) -> bool) -> usize {
        let removed = self.settings.remove_repo(condition);

        join_all(removed.iter().map(|x| async move {
            match remove_file(x.local_path()) {
                Ok(()) => (),
                Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                Err(e) => error!(
                    "removing local repository {}: {}",
                    x.local_path().display(),
                    e
                ),
            }
        }))
        .await;

        removed.len()
    }

    pub async fn cleanup_repos_folder(&self, io: &impl EnvironmentIo) -> io::Result<()> {
        let mut uesr_repo_file_names = HashSet::<OsString>::from_iter([
            OsString::from("vrc-official.json"),
            OsString::from("vrc-curated.json"),
            OsString::from("package-cache.json"),
        ]);
        let repos_base = io.resolve(REPO_CACHE_FOLDER.as_ref());

        for x in self.get_user_repos() {
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
                let mut path =
                    OsString::with_capacity(REPO_CACHE_FOLDER.len() + 1 + file_name.len());
                path.push(REPO_CACHE_FOLDER);
                path.push(OsStr::new("/"));
                path.push(file_name);
                io.remove_file(path.as_ref()).await?;
            }
        }

        Ok(())
    }

    pub async fn clear_package_cache(&self, io: &impl EnvironmentIo) -> io::Result<()> {
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

    pub fn export_repositories(&self) -> String {
        let mut builder = String::new();

        for setting in self.get_user_repos() {
            let Some(url) = setting.url() else { continue };
            if setting.headers().is_empty() {
                writeln!(builder, "{url}").unwrap();
            } else {
                let mut add_url = Url::parse("vcc://vpm/addRepo").unwrap();
                let mut query_builder = add_url.query_pairs_mut();
                query_builder.clear();
                query_builder.append_pair("url", url.as_str());

                for (header_name, value) in setting.headers() {
                    query_builder.append_pair("headers[]", &format!("{}:{}", header_name, value));
                }
                drop(query_builder);

                writeln!(builder, "{}", add_url).unwrap();
            }
        }

        builder
    }

    pub fn show_prerelease_packages(&self) -> bool {
        self.settings.show_prerelease_packages()
    }

    pub fn set_show_prerelease_packages(&mut self, value: bool) {
        self.settings.set_show_prerelease_packages(value)
    }

    pub fn default_project_path(&self) -> Option<&str> {
        self.settings.default_project_path()
    }

    pub fn set_default_project_path(&mut self, value: &str) {
        self.settings.set_default_project_path(value)
    }

    pub fn project_backup_path(&self) -> Option<&str> {
        self.settings.project_backup_path()
    }

    pub fn set_project_backup_path(&mut self, value: &str) {
        self.settings.set_project_backup_path(value)
    }

    pub fn unity_hub_path(&self) -> &str {
        self.settings.unity_hub_path()
    }

    pub fn set_unity_hub_path(&mut self, value: &str) {
        self.settings.set_unity_hub_path(value)
    }

    pub fn ignore_curated_repository(&self) -> bool {
        self.settings.ignore_curated_repository()
    }

    pub fn ignore_official_repository(&self) -> bool {
        self.settings.ignore_official_repository()
    }
}

pub enum AddUserPackageResult {
    Success,
    NonAbsolute,
    BadPackage,
    AlreadyAdded,
}

impl Environment {
    pub fn user_packages(&self) -> &[(PathBuf, PackageManifest)] {
        &self.collection.user_packages
    }

    pub fn remove_user_package(&mut self, pkg_path: &Path) {
        self.settings.remove_user_package(pkg_path);
    }

    pub async fn add_user_package(
        &mut self,
        pkg_path: &Path,
        io: &impl EnvironmentIo,
    ) -> AddUserPackageResult {
        self.settings.add_user_package(pkg_path, io).await
    }
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

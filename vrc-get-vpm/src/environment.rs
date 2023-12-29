mod empty;
mod repo_holder;
mod repo_source;
mod settings;
mod uesr_package_collection;

use crate::repository::local::LocalCachedRepository;
use crate::repository::{RemotePackages, RemoteRepository};
use crate::structs::package::PackageJson;
use crate::structs::setting::UserRepoSetting;
use crate::traits::{HttpClient, PackageCollection, RemotePackageDownloader};
use crate::utils::{PathBufExt, Sha256AsyncWrite};
use crate::{PackageInfo, VersionSelector};
use either::{Left, Right};
use enum_map::EnumMap;
use futures::future::join_all;
use hex::FromHex;
use indexmap::IndexMap;
use itertools::Itertools;
use log::error;
use serde_json::to_vec_pretty;
use std::cmp::Reverse;
use std::collections::HashSet;
use std::fs::remove_file;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::{env, fmt, io};
use tokio::fs::{create_dir_all, File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_util::compat::*;
use url::Url;

use crate::environment::repo_source::{PreDefinedRepoType, PredefinedSource};
pub use empty::EmptyEnvironment;
pub(crate) use repo_holder::RepoHolder;
pub(crate) use repo_source::RepoSource;
pub(crate) use settings::Settings;
pub(crate) use uesr_package_collection::UserPackageCollection;

/// This struct holds global state (will be saved on %LOCALAPPDATA% of VPM.
#[derive(Debug)]
pub struct Environment<T: HttpClient> {
    pub(crate) http: Option<T>,
    /// config folder.
    /// On windows, `%APPDATA%\\VRChatCreatorCompanion`.
    /// On posix, `${XDG_DATA_HOME}/VRChatCreatorCompanion`.
    pub(crate) global_dir: PathBuf,
    /// parsed settings
    settings: Settings,
    /// Cache
    repo_cache: RepoHolder,
    user_packages: UserPackageCollection,
    predefined_repos: EnumMap<PreDefinedRepoType, PredefinedSource>,
}

impl<T: HttpClient> Environment<T> {
    pub async fn load_default(http: Option<T>) -> io::Result<Self> {
        let mut folder = Self::get_local_config_folder();
        folder.push("VRChatCreatorCompanion");
        let folder = folder;

        log::debug!(
            "initializing Environment with config folder {}",
            folder.display()
        );

        Ok(Self {
            http,
            settings: Settings::load(folder.join("settings.json")).await?,
            repo_cache: RepoHolder::new(),
            user_packages: UserPackageCollection::new(),
            predefined_repos: EnumMap::from_fn(|x: PreDefinedRepoType| {
                PredefinedSource::new(&folder, x)
            }),
            global_dir: folder,
        })
    }

    #[cfg(windows)]
    fn get_local_config_folder() -> PathBuf {
        return dirs_sys::known_folder_local_app_data().expect("LocalAppData not found");
    }

    #[cfg(not(windows))]
    fn get_local_config_folder() -> PathBuf {
        if let Some(data_home) = env::var_os("XDG_DATA_HOME") {
            log::debug!("XDG_DATA_HOME found {:?}", data_home);
            return data_home.into();
        }

        // fallback: use HOME
        if let Some(home_folder) = env::var_os("HOME") {
            log::debug!("HOME found {:?}", home_folder);
            let mut path = PathBuf::from(home_folder);
            path.push(".local/share");
            return path;
        }

        panic!("no XDG_DATA_HOME nor HOME are set!")
    }
}

impl<T: HttpClient> Environment<T> {
    pub async fn load_package_infos(&mut self, update: bool) -> io::Result<()> {
        let http = if update { self.http.as_ref() } else { None };
        let predefined_repos = self.predefined_repos.values();
        let user_repos = self.settings.user_repos().iter();
        self.repo_cache
            .load_repos(
                http,
                predefined_repos.map(Left).chain(user_repos.map(Right)),
            )
            .await?;
        self.update_user_repo_id();
        self.load_user_package_infos().await?;
        self.remove_id_duplication();
        Ok(())
    }

    fn update_user_repo_id(&mut self) {
        // update id field
        struct NewIdGetterImpl<'b>(&'b RepoHolder);

        impl<'b> settings::NewIdGetter for NewIdGetterImpl<'b> {
            fn new_id<'a>(&'a self, repo: &'a UserRepoSetting) -> Option<&'a str> {
                let loaded = self.0.get_repo(repo.local_path()).unwrap();

                let id = loaded.id();
                let url = loaded.url().map(Url::as_str);
                let local_url = repo.url().map(Url::as_str);

                id.or(url).or(local_url)
            }
        }

        self.settings
            .update_user_repo_id(NewIdGetterImpl(&self.repo_cache));
    }

    fn remove_id_duplication(&mut self) {
        let user_repos = self.get_user_repos();
        if user_repos.is_empty() {
            return;
        }

        let mut used_ids = HashSet::new();

        // retain operates in place, visiting each element exactly once in the original order.
        // s
        self.settings.retain_user_repos(|repo| {
            let mut to_add = true;
            if let Some(id) = repo.id() {
                to_add = used_ids.insert(id.to_owned());
            }
            if to_add {
                // this means new id
                true
            } else {
                // this means duplicated id: removed so mark as changed
                self.repo_cache.remove_repo(repo.local_path());

                false
            }
        });
    }

    async fn load_user_package_infos(&mut self) -> io::Result<()> {
        self.user_packages.clear();
        for x in self.settings.user_package_folders() {
            self.user_packages.try_add_package(x).await?;
        }
        Ok(())
    }

    pub fn get_repos_dir(&self) -> PathBuf {
        self.global_dir.join("Repos")
    }
}

impl<T: HttpClient> PackageCollection for Environment<T> {
    fn get_all_packages(&self) -> impl Iterator<Item = PackageInfo> {
        self.repo_cache
            .get_all_packages()
            .chain(self.user_packages.get_all_packages())
    }

    fn find_packages(&self, package: &str) -> impl Iterator<Item = PackageInfo> {
        self.repo_cache
            .find_packages(package)
            .chain(self.user_packages.find_packages(package))
    }

    fn find_package_by_name(
        &self,
        package: &str,
        package_selector: VersionSelector,
    ) -> Option<PackageInfo> {
        let local = self
            .repo_cache
            .find_package_by_name(package, package_selector);
        let user = self
            .user_packages
            .find_package_by_name(package, package_selector);

        return local.into_iter().chain(user).max_by_key(|x| x.version());
    }
}

impl<T: HttpClient> Environment<T> {
    pub fn get_repos(&self) -> impl Iterator<Item = (&'_ PathBuf, &'_ LocalCachedRepository)> {
        self.repo_cache.get_repo_with_path()
    }

    pub fn find_whole_all_packages(
        &self,
        filter: impl Fn(&PackageJson) -> bool,
    ) -> Vec<&PackageJson> {
        let mut list = Vec::new();

        self.get_repos()
            .flat_map(|(_, repo)| repo.get_packages())
            .filter_map(RemotePackages::get_latest)
            .filter(|x| filter(x))
            .fold((), |_, pkg| list.push(pkg));

        // user package folders
        for info in self.user_packages.get_all_packages() {
            if !info.version().pre.is_empty() && filter(info.package_json()) {
                list.push(info.package_json());
            }
        }

        list.sort_by_key(|x| Reverse(x.version()));

        list.into_iter()
            .unique_by(|x| (x.name(), x.version()))
            .collect()
    }

    pub fn get_user_repos(&self) -> &[UserRepoSetting] {
        self.settings.user_repos()
    }

    pub async fn add_remote_repo(
        &mut self,
        url: Url,
        name: Option<&str>,
        headers: IndexMap<String, String>,
    ) -> Result<(), AddRepositoryErr> {
        let user_repos = self.get_user_repos();
        if user_repos.iter().any(|x| x.url() == Some(&url)) {
            return Err(AddRepositoryErr::AlreadyAdded);
        }
        let http = self.http.as_ref().ok_or(AddRepositoryErr::OfflineMode)?;

        let (remote_repo, etag) = RemoteRepository::download(http, &url, &headers).await?;
        let repo_name = name.or(remote_repo.name()).map(str::to_owned);

        let repo_id = remote_repo.id().map(str::to_owned);

        if let Some(repo_id) = repo_id.as_deref() {
            // if there is id, check if there is already repo with same id
            if user_repos.iter().any(|x| x.id() == Some(repo_id)) {
                return Err(AddRepositoryErr::AlreadyAdded);
            }
        }

        let mut local_cache = LocalCachedRepository::new(remote_repo, headers);

        // set etag
        if let Some(etag) = etag {
            local_cache
                .vrc_get
                .get_or_insert_with(Default::default)
                .etag = etag;
        }

        create_dir_all(self.get_repos_dir()).await?;

        let local_path = self.write_new_repo(&local_cache).await?;

        self.settings.add_user_repo(UserRepoSetting::new(
            local_path.clone(),
            repo_name,
            Some(url),
            repo_id,
        ));
        Ok(())
    }

    async fn write_new_repo(&self, local_cache: &LocalCachedRepository) -> io::Result<PathBuf> {
        create_dir_all(self.get_repos_dir()).await?;

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
            let path = self.get_repos_dir().joined(file_name);
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .await
            {
                Ok(mut file) => {
                    file.write_all(&to_vec_pretty(&local_cache)?).await?;
                    file.flush().await?;

                    return Ok(path);
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
        if self.get_user_repos().iter().any(|x| x.local_path() == path) {
            return Err(AddRepositoryErr::AlreadyAdded);
        }

        self.settings.add_user_repo(UserRepoSetting::new(
            path.to_owned(),
            name.map(str::to_owned),
            None,
            None,
        ));
        Ok(())
    }

    pub async fn remove_repo(&mut self, condition: impl Fn(&UserRepoSetting) -> bool) -> usize {
        let removed = self.settings.retain_user_repos(|x| !condition(x));

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

    #[cfg(feature = "experimental-override-predefined")]
    pub fn set_official_url_override(&mut self, url: Url) {
        self.predefined_repos[PreDefinedRepoType::Official].url = url;
    }

    #[cfg(feature = "experimental-override-predefined")]
    pub fn set_curated_url_override(&mut self, url: Url) {
        self.predefined_repos[PreDefinedRepoType::Curated].url = url;
    }

    pub async fn save(&mut self) -> io::Result<()> {
        self.settings.save().await
    }
}

impl<T: HttpClient> RemotePackageDownloader for Environment<T> {
    async fn get_package(
        &self,
        repository: &LocalCachedRepository,
        package: &PackageJson,
    ) -> io::Result<File> {
        let zip_file_name = format!("vrc-get-{}-{}.zip", &package.name(), package.version());
        let zip_path = self
            .global_dir
            .to_owned()
            .joined("Repos")
            .joined(package.name())
            .joined(&zip_file_name);
        let sha_path = zip_path.with_extension("zip.sha256");

        if let Some(cache_file) = try_load_package_cache(&zip_path, &sha_path, None).await {
            Ok(cache_file)
        } else {
            create_dir_all(zip_path.parent().unwrap()).await?;

            Ok(download_package_zip(
                self.http.as_ref(),
                repository.headers(),
                &zip_path,
                &sha_path,
                &zip_file_name,
                package.url().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "URL field of the package.json in the repository empty",
                    )
                })?,
            )
            .await?)
        }
    }
}

/// Try to load from the zip file
///
/// # Arguments
///
/// * `zip_path`: the path to zip file
/// * `sha_path`: the path to sha256 file
/// * `sha256`: sha256 hash if specified
///
/// returns: Option<File> readable zip file file or None
async fn try_load_package_cache(
    zip_path: &Path,
    sha_path: &Path,
    sha256: Option<&str>,
) -> Option<File> {
    let mut cache_file = File::open(zip_path).await.ok()?;

    let mut buf = [0u8; 256 / 4];
    File::open(sha_path)
        .await
        .ok()?
        .read_exact(&mut buf)
        .await
        .ok()?;

    let hex: [u8; 256 / 8] = FromHex::from_hex(buf).ok()?;

    // is stored sha doesn't match sha in repo: current cache is invalid
    if let Some(repo_hash) = sha256.and_then(|x| <[u8; 256 / 8] as FromHex>::from_hex(x).ok()) {
        if repo_hash != hex {
            return None;
        }
    }

    let mut hasher = Sha256AsyncWrite::new(tokio::io::sink());

    tokio::io::copy(&mut cache_file, &mut hasher).await.ok()?;

    let hash = &hasher.finalize().1[..];
    if hash != &hex[..] {
        return None;
    }

    cache_file.seek(SeekFrom::Start(0)).await.ok()?;

    Some(cache_file)
}

/// downloads the zip file from the url to the specified path
///
/// # Arguments
///
/// * `http`: http client. returns error if none
/// * `zip_path`: the path to zip file
/// * `sha_path`: the path to sha256 file
/// * `zip_file_name`: the name of zip file. will be used in the sha file
/// * `url`: url to zip file
///
/// returns: Result<File, Error> the readable zip file.
async fn download_package_zip(
    http: Option<&impl HttpClient>,
    headers: &IndexMap<String, String>,
    zip_path: &Path,
    sha_path: &Path,
    zip_file_name: &str,
    url: &Url,
) -> io::Result<File> {
    let Some(http) = http else {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Offline mode"));
    };

    // file not found: err
    let cache_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&zip_path)
        .await?;

    let mut response = pin!(http.get(url, headers).await?.compat());

    let mut writer = Sha256AsyncWrite::new(cache_file);
    tokio::io::copy(&mut response, &mut writer).await?;

    let (mut cache_file, hash) = writer.finalize();

    cache_file.flush().await?;
    cache_file.seek(SeekFrom::Start(0)).await?;

    // write sha file
    tokio::fs::write(
        &sha_path,
        format!("{} {}\n", hex::encode(&hash[..]), zip_file_name),
    )
    .await?;

    Ok(cache_file)
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

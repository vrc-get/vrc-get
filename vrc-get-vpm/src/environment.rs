mod repo_holder;
mod repo_source;
mod settings;
mod uesr_package_collection;
mod vrc_get_settings;

#[cfg(feature = "vrc-get-litedb")]
mod litedb;
#[cfg(feature = "experimental-project-management")]
mod project_management;
#[cfg(feature = "experimental-unity-management")]
mod unity_management;

use crate::io;
use crate::io::SeekFrom;
use crate::repository::local::LocalCachedRepository;
use crate::repository::RemoteRepository;
use crate::structs::setting::UserRepoSetting;
use crate::traits::{EnvironmentIoHolder, HttpClient, PackageCollection, RemotePackageDownloader};
use crate::utils::{normalize_path, to_vec_pretty_os_eol, Sha256AsyncWrite};
use crate::{PackageInfo, PackageManifest, VersionSelector};
use futures::future::{join_all, try_join};
use futures::prelude::*;
use hex::FromHex;
use indexmap::IndexMap;
use itertools::Itertools;
use lazy_static::lazy_static;
use log::{error, warn};
use std::cmp::Reverse;
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::remove_file;
use std::path::{Path, PathBuf};
use std::pin::pin;
use url::Url;

use crate::environment::vrc_get_settings::VrcGetSettings;
use crate::io::{DirEntry, EnvironmentIo};
#[cfg(feature = "experimental-project-management")]
pub use project_management::*;
pub(crate) use repo_holder::RepoHolder;
pub(crate) use repo_source::RepoSource;
pub(crate) use settings::Settings;
pub(crate) use uesr_package_collection::UserPackageCollection;

const OFFICIAL_URL_STR: &str = "https://packages.vrchat.com/official?download";
const LOCAL_OFFICIAL_PATH: &str = "Repos/vrc-official.json";
const CURATED_URL_STR: &str = "https://packages.vrchat.com/curated?download";
const LOCAL_CURATED_PATH: &str = "Repos/vrc-curated.json";
const REPO_CACHE_FOLDER: &str = "Repos";

/// This struct holds global state (will be saved on %LOCALAPPDATA% of VPM.
#[derive(Debug)]
pub struct Environment<T: HttpClient, IO: EnvironmentIo> {
    pub(crate) http: Option<T>,
    pub(crate) io: IO,
    /// parsed settings
    settings: Settings,
    vrc_get_settings: VrcGetSettings,
    // we do not connect to litedb unless we need information from litedb.
    // TODO?: use inner mutability?
    #[cfg(feature = "vrc-get-litedb")]
    litedb_connection: litedb::LiteDbConnectionHolder,
    /// Cache
    repo_cache: RepoHolder,
    user_packages: UserPackageCollection,
}

impl<T: HttpClient, IO: EnvironmentIo> Environment<T, IO> {
    pub async fn load(http: Option<T>, io: IO) -> io::Result<Self> {
        Ok(Self {
            http,
            settings: Settings::load(&io).await?,
            vrc_get_settings: VrcGetSettings::load(&io).await?,
            #[cfg(feature = "vrc-get-litedb")]
            litedb_connection: litedb::LiteDbConnectionHolder::new(),
            repo_cache: RepoHolder::new(),
            user_packages: UserPackageCollection::new(),
            io,
        })
    }

    /// Reload configuration files on the filesystem.
    /// This doesn't update repository cache or user package cache.
    /// Please call [`load_package_infos`] after this method.
    ///
    /// [`load_package_infos`]: Environment::load_package_infos
    pub async fn reload(&mut self) -> io::Result<()> {
        self.settings = Settings::load(&self.io).await?;
        self.vrc_get_settings = VrcGetSettings::load(&self.io).await?;
        #[cfg(feature = "vrc-get-litedb")]
        {
            self.litedb_connection = litedb::LiteDbConnectionHolder::new();
        }
        Ok(())
    }
}

impl<T: HttpClient, IO: EnvironmentIo> Environment<T, IO> {
    fn get_predefined_repos(&self) -> Vec<RepoSource<'static>> {
        lazy_static! {
            static ref EMPTY_HEADERS: IndexMap<Box<str>, Box<str>> = IndexMap::new();
            static ref OFFICIAL_URL: Url = Url::parse(OFFICIAL_URL_STR).unwrap();
            static ref CURATED_URL: Url = Url::parse(CURATED_URL_STR).unwrap();
        }

        let mut repositories = Vec::with_capacity(2);

        if !self.vrc_get_settings.ignore_official_repository() {
            repositories.push(RepoSource::new(
                LOCAL_OFFICIAL_PATH.as_ref(),
                &EMPTY_HEADERS,
                Some(&OFFICIAL_URL),
            ));
        } else {
            warn!("ignoring official repository is experimental feature!");
        }

        if !self.vrc_get_settings.ignore_curated_repository() {
            repositories.push(RepoSource::new(
                LOCAL_CURATED_PATH.as_ref(),
                &EMPTY_HEADERS,
                Some(&CURATED_URL),
            ));
        } else {
            warn!("ignoring curated repository is experimental feature!");
        }

        repositories
    }

    pub async fn load_package_infos(&mut self, update: bool) -> io::Result<()> {
        let http = if update { self.http.as_ref() } else { None };
        let predefined_repos = self.get_predefined_repos().into_iter();
        let user_repos = self
            .settings
            .user_repos()
            .iter()
            .map(UserRepoSetting::to_source);
        self.io.create_dir_all("Repos".as_ref()).await?;
        self.repo_cache
            .load_repos(http, &self.io, predefined_repos.chain(user_repos))
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
            fn new_id<'a>(&'a self, repo: &'a UserRepoSetting) -> Result<Option<&'a str>, ()> {
                let loaded = self.0.get_repo(repo.local_path()).ok_or(())?;

                let id = loaded.id();
                let url = loaded.url().map(Url::as_str);
                let local_url = repo.url().map(Url::as_str);

                Ok(id.or(url).or(local_url))
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
            self.user_packages.try_add_package(&self.io, x).await;
        }
        Ok(())
    }
}

impl<T: HttpClient, IO: EnvironmentIo> PackageCollection for Environment<T, IO> {
    fn get_curated_packages(
        &self,
        version_selector: VersionSelector,
    ) -> impl Iterator<Item = PackageInfo> {
        self.repo_cache
            .get_repo(LOCAL_CURATED_PATH.as_ref())
            .map(move |repo| {
                repo.repo()
                    .get_packages()
                    .filter_map(move |x| x.get_latest(version_selector))
                    .map(|json| PackageInfo::remote(json, repo))
            })
            .into_iter()
            .flatten()
    }

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

impl<T: HttpClient, IO: EnvironmentIo> EnvironmentIoHolder for Environment<T, IO> {
    type EnvironmentIo = IO;

    fn io(&self) -> &Self::EnvironmentIo {
        &self.io
    }
}

impl<T: HttpClient, IO: EnvironmentIo> Environment<T, IO> {
    pub fn get_repos(&self) -> impl Iterator<Item = (&'_ Box<Path>, &'_ LocalCachedRepository)> {
        self.repo_cache.get_repo_with_path()
    }

    pub fn find_whole_all_packages(
        &self,
        version_selector: VersionSelector,
        filter: impl Fn(&PackageManifest) -> bool,
    ) -> Vec<PackageInfo> {
        let mut list = Vec::new();

        self.get_repos()
            .flat_map(|(_, repo)| {
                repo.get_packages()
                    .filter_map(|packages| packages.get_latest(version_selector))
                    .map(|json| PackageInfo::remote(json, repo))
            })
            .filter(|x| filter(x.package_json()))
            .fold((), |_, pkg| list.push(pkg));

        // user package folders
        for info in self.user_packages.get_all_packages() {
            if !info.version().pre.is_empty() && filter(info.package_json()) {
                list.push(info);
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
        headers: IndexMap<Box<str>, Box<str>>,
    ) -> Result<(), AddRepositoryErr> {
        let user_repos = self.get_user_repos();
        if user_repos.iter().any(|x| x.url() == Some(&url)) {
            return Err(AddRepositoryErr::AlreadyAdded);
        }
        // should we check more urls?
        if !self.ignore_curated_repository()
            && url.as_str() == "https://packages.vrchat.com/curated?download"
        {
            return Err(AddRepositoryErr::AlreadyAdded);
        }
        if !self.ignore_official_repository()
            && url.as_str() == "https://packages.vrchat.com/official?download"
        {
            return Err(AddRepositoryErr::AlreadyAdded);
        }

        let http = self.http.as_ref().ok_or(AddRepositoryErr::OfflineMode)?;

        let (remote_repo, etag) = RemoteRepository::download(http, &url, &headers).await?;
        let repo_name = name.or(remote_repo.name()).map(Into::into);

        let repo_id = remote_repo.id().map(Into::into);

        if let Some(repo_id) = repo_id.as_deref() {
            // if there is id, check if there is already repo with same id
            if user_repos.iter().any(|x| x.id() == Some(repo_id)) {
                return Err(AddRepositoryErr::AlreadyAdded);
            }
            if repo_id == "com.vrchat.repos.official"
                && !self.vrc_get_settings.ignore_official_repository()
            {
                return Err(AddRepositoryErr::AlreadyAdded);
            }
            if repo_id == "com.vrchat.repos.curated"
                && !self.vrc_get_settings.ignore_curated_repository()
            {
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

        self.io.create_dir_all(REPO_CACHE_FOLDER.as_ref()).await?;

        let file_name = self.write_new_repo(&local_cache).await?;

        self.settings.add_user_repo(UserRepoSetting::new(
            self.io
                .resolve(format!("{}/{}", REPO_CACHE_FOLDER, file_name).as_ref())
                .into_boxed_path(),
            repo_name,
            Some(url),
            repo_id,
        ));
        Ok(())
    }

    async fn write_new_repo(&self, local_cache: &LocalCachedRepository) -> io::Result<String> {
        self.io.create_dir_all(REPO_CACHE_FOLDER.as_ref()).await?;

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
            match self
                .io
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
        let path = normalize_path(path);

        if self.get_user_repos().iter().any(|x| x.local_path() == path) {
            return Err(AddRepositoryErr::AlreadyAdded);
        }

        self.settings.add_user_repo(UserRepoSetting::new(
            path.into(),
            name.map(Into::into),
            None,
            None,
        ));
        Ok(())
    }

    pub async fn remove_repo(&mut self, condition: impl Fn(&UserRepoSetting) -> bool) -> usize {
        let removed = self.settings.retain_user_repos(|x| !condition(x));

        for x in &removed {
            self.repo_cache.remove_repo(x.local_path());
        }

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

    pub async fn cleanup_repos_folder(&self) -> io::Result<()> {
        let mut uesr_repo_file_names = HashSet::<OsString>::from_iter([
            OsString::from("vrc-official.json"),
            OsString::from("vrc-curated.json"),
            OsString::from("package-cache.json"),
        ]);
        let repos_base = self.io.resolve(REPO_CACHE_FOLDER.as_ref());

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

        let mut entry = self.io.read_dir(REPO_CACHE_FOLDER.as_ref()).await?;
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
                self.io.remove_file(path.as_ref()).await?;
            }
        }

        Ok(())
    }

    pub async fn clear_package_cache(&self) -> io::Result<()> {
        let io = &self.io;

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
                        {
                            if cache_entry.file_type().await?.is_file() {
                                return Ok(Some((pkg_id, cache_entry)));
                            }
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

    pub fn show_prerelease_packages(&self) -> bool {
        self.settings.show_prerelease_packages()
    }

    pub fn set_show_prerelease_packages(&mut self, value: bool) {
        self.settings.set_show_prerelease_packages(value);
    }

    pub fn default_project_path(&self) -> &str {
        self.settings.default_project_path()
    }

    pub fn set_default_project_path(&mut self, value: &str) {
        self.settings.set_default_project_path(value);
    }

    pub fn project_backup_path(&self) -> &str {
        self.settings.project_backup_path()
    }

    pub fn set_project_backup_path(&mut self, value: &str) {
        self.settings.set_project_backup_path(value);
    }

    pub fn unity_hub_path(&self) -> &str {
        self.settings.unity_hub()
    }

    pub fn set_unity_hub_path(&mut self, value: &str) {
        self.settings.set_unity_hub(value);
    }

    pub fn ignore_curated_repository(&self) -> bool {
        self.vrc_get_settings.ignore_curated_repository()
    }

    pub fn ignore_official_repository(&self) -> bool {
        self.vrc_get_settings.ignore_official_repository()
    }

    pub fn http(&self) -> Option<&T> {
        self.http.as_ref()
    }

    pub async fn save(&mut self) -> io::Result<()> {
        try_join(
            self.settings.save(&self.io),
            self.vrc_get_settings.save(&self.io),
        )
        .await
        .map(|_| ())?;

        #[cfg(feature = "vrc-get-litedb")]
        self.disconnect_litedb();
        Ok(())
    }
}

impl<T: HttpClient, IO: EnvironmentIo> RemotePackageDownloader for Environment<T, IO> {
    type FileStream = IO::FileStream;

    async fn get_package(
        &self,
        repository: &LocalCachedRepository,
        package: &PackageManifest,
    ) -> io::Result<Self::FileStream> {
        let zip_file_name = format!("vrc-get-{}-{}.zip", &package.name(), package.version());
        let zip_path = PathBuf::from(format!(
            "{}/{}/{}",
            REPO_CACHE_FOLDER,
            package.name(),
            &zip_file_name
        ));
        let sha_path = zip_path.with_extension("zip.sha256");

        if let Some(cache_file) =
            try_load_package_cache(&self.io, &zip_path, &sha_path, package.zip_sha_256()).await
        {
            Ok(cache_file)
        } else {
            self.io.create_dir_all(zip_path.parent().unwrap()).await?;

            let new_headers = IndexMap::from_iter(
                (repository
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.as_ref(), v.as_ref())))
                .chain(
                    package
                        .headers()
                        .iter()
                        .map(|(k, v)| (k.as_ref(), v.as_ref())),
                ),
            );

            let (zip_file, zip_hash) = download_package_zip(
                self.http.as_ref(),
                &self.io,
                &new_headers,
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
            .await?;

            if let Some(repo_hash) = package
                .zip_sha_256()
                .and_then(|x| <[u8; 256 / 8] as FromHex>::from_hex(x).ok())
            {
                if repo_hash != zip_hash {
                    error!(
                        "Package hash mismatched! This will be hard error in the future!: {} v{}",
                        package.name(),
                        package.version()
                    );
                    //return None;
                }
            }

            Ok(zip_file)
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
/// returns: Option<File> readable zip file or None
async fn try_load_package_cache<IO: EnvironmentIo>(
    io: &IO,
    zip_path: &Path,
    sha_path: &Path,
    sha256: Option<&str>,
) -> Option<IO::FileStream> {
    let mut cache_file = io.open(zip_path).await.ok()?;

    let mut buf = [0u8; 256 / 4];
    io.open(sha_path)
        .await
        .ok()?
        .read_exact(&mut buf)
        .await
        .ok()?;

    let hex: [u8; 256 / 8] = FromHex::from_hex(buf).ok()?;

    // if stored sha doesn't match sha in repo: current cache is invalid
    if let Some(repo_hash) = sha256.and_then(|x| <[u8; 256 / 8] as FromHex>::from_hex(x).ok()) {
        if repo_hash != hex {
            return None;
        }
    }

    let mut hasher = Sha256AsyncWrite::new(io::sink());

    io::copy(&mut cache_file, &mut hasher).await.ok()?;

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
async fn download_package_zip<IO: EnvironmentIo>(
    http: Option<&impl HttpClient>,
    io: &IO,
    headers: &IndexMap<&str, &str>,
    zip_path: &Path,
    sha_path: &Path,
    zip_file_name: &str,
    url: &Url,
) -> io::Result<(IO::FileStream, [u8; 256 / 8])> {
    let Some(http) = http else {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Offline mode"));
    };

    // file not found: err
    let cache_file = io.create(zip_path).await?;

    let mut response = pin!(http.get(url, headers).await?);

    let mut writer = Sha256AsyncWrite::new(cache_file);
    io::copy(&mut response, &mut writer).await?;

    let (mut cache_file, hash) = writer.finalize();
    let hash: [u8; 256 / 8] = hash.into();

    cache_file.flush().await?;
    cache_file.seek(SeekFrom::Start(0)).await?;

    // write sha file
    io.write(
        sha_path,
        format!("{} {}\n", hex::encode(&hash[..]), zip_file_name).as_bytes(),
    )
    .await?;

    Ok((cache_file, hash))
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

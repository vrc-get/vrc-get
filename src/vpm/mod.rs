//! This module contains vpm core implementation
//!
//! This module might be a separated crate.

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::future::ready;
use std::io::SeekFrom;
use std::path::{Component, Path, PathBuf};
use std::task::ready;
use std::task::Poll::Ready;
use std::{env, fmt, io};

use futures::future::{join_all, try_join_all};
use futures::prelude::*;
use indexmap::IndexMap;
use itertools::{Itertools as _, sorted};
use reqwest::{Client, IntoUrl, Url};
use serde_json::{from_value, to_value, Map, Value};
use tokio::fs::{
    create_dir_all, read_dir, remove_dir_all, remove_file, DirEntry, File, OpenOptions,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use repo_holder::RepoHolder;
use utils::*;
use vpm_manifest::VpmManifest;

use crate::version::{Version, VersionRange};
use crate::vpm::structs::manifest::{VpmDependency, VpmLockedDependency};
use crate::vpm::structs::package::PackageJson;
use crate::vpm::structs::remote_repo::PackageVersions;
use crate::vpm::structs::repository::LocalCachedRepository;
use crate::vpm::structs::setting::UserRepoSetting;
use sha2::{Digest, Sha256};

mod repo_holder;
pub mod structs;
mod utils;

type JsonMap = Map<String, Value>;

/// This struct holds global state (will be saved on %LOCALAPPDATA% of VPM.
#[derive(Debug)]
pub struct Environment {
    http: Option<Client>,
    /// config folder.
    /// On windows, `%APPDATA%\\VRChatCreatorCompanion`.
    /// On posix, `${XDG_DATA_HOME}/VRChatCreatorCompanion`.
    global_dir: PathBuf,
    /// parsed settings
    settings: Map<String, Value>,
    /// Cache
    repo_cache: RepoHolder,
    settings_changed: bool,
}

impl Environment {
    pub async fn load_default(http: Option<Client>) -> io::Result<Environment> {
        let mut folder = Environment::get_local_config_folder();
        folder.push("VRChatCreatorCompanion");
        let folder = folder;

        log::debug!(
            "initializing Environment with config folder {}",
            folder.display()
        );

        Ok(Environment {
            http: http.clone(),
            settings: load_json_or_default(&folder.join("settings.json")).await?,
            global_dir: folder,
            repo_cache: RepoHolder::new(http),
            settings_changed: false,
        })
    }

    #[cfg(windows)]
    fn get_local_config_folder() -> PathBuf {
        use std::ffi::c_void;
        use std::os::windows::ffi::OsStringExt;
        use windows::core::{GUID, PWSTR};
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::UI::Shell::KNOWN_FOLDER_FLAG;

        // due to intellij rust bug, windows::Win32::UI::Shell::SHGetKnownFolderPath is not shown
        // so I write wrapper here
        #[allow(non_snake_case)]
        #[inline(always)]
        pub unsafe fn SHGetKnownFolderPath(
            rfid: *const GUID,
            dwflags: KNOWN_FOLDER_FLAG,
            htoken: HANDLE,
        ) -> windows::core::Result<PWSTR>
        {
            windows::Win32::UI::Shell::SHGetKnownFolderPath(rfid, dwflags, htoken)
        }

        let path = unsafe {
            let path = SHGetKnownFolderPath(
                &windows::Win32::UI::Shell::FOLDERID_LocalAppData,
                KNOWN_FOLDER_FLAG(0),
                HANDLE::default(),
            )
                .expect("cannot get Local AppData folder");
            let os_string = OsString::from_wide(path.as_wide());
            windows::Win32::System::Com::CoTaskMemFree(Some(path.as_ptr().cast::<c_void>()));
            os_string
        };

        return PathBuf::from(path);
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

    pub(crate) fn get_repos_dir(&self) -> PathBuf {
        self.global_dir.join("Repos")
    }

    pub async fn find_package_by_name<'a>(
        &self,
        package: &str,
        version: VersionSelector<'a>,
    ) -> io::Result<Option<PackageJson>> {
        let mut versions = self.find_packages(package).await?;

        versions.retain(|x| version.satisfies(&x.version));

        versions.sort_by(|a, b| a.version.cmp(&b.version).reverse());

        Ok(versions.into_iter().next())
    }

    pub async fn get_repo_sources(&self) -> io::Result<Vec<RepoSource>> {
        // collect user repositories for get_repos_dir
        let repos_base = self.get_repos_dir();
        let user_repos = self.get_user_repos()?;

        let mut user_repo_file_names = HashSet::new();
        user_repo_file_names.insert(OsStr::new("vrc-curated.json"));
        user_repo_file_names.insert(OsStr::new("vrc-official.json"));

        fn relative_file_name<'a>(path: &'a Path, base: &Path) -> Option<&'a OsStr> {
            path.strip_prefix(&base)
                .ok()
                .filter(|x| x.parent().map(|x| x.as_os_str().is_empty()).unwrap_or(true))
                .and_then(|x| x.file_name())
        }

        user_repo_file_names.extend(
            user_repos
                .iter()
                .filter_map(|x| relative_file_name(&x.local_path, &repos_base)),
        );

        let mut entry = match read_dir(self.get_repos_dir()).await {
            Ok(entry) => Some(entry),
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        let streams = stream::poll_fn(|cx| {
            Ready(match entry {
                Some(ref mut entry) => match ready!(entry.poll_next_entry(cx)) {
                    Ok(Some(v)) => Some(Ok(v)),
                    Ok(None) => None,
                    Err(e) => Some(Err(e)),
                },
                None => None,
            })
        });

        let undefined_repos = streams
            .map_ok(|x| x.path())
            .try_filter(|x| ready(!user_repo_file_names.contains(x.file_name().unwrap())))
            .try_filter(|x| ready(x.extension() == Some(OsStr::new("json"))))
            .try_filter(|x| {
                tokio::fs::metadata(x.clone()).map(|x| x.map(|x| x.is_file()).unwrap_or(false))
            })
            .map_ok(RepoSource::Undefined);

        let defined_sources = DEFINED_REPO_SOURCES
            .into_iter()
            .copied()
            .map(RepoSource::PreDefined);
        let user_repo_sources = self.get_user_repos()?.into_iter().map(RepoSource::UserRepo);

        stream::iter(defined_sources.chain(user_repo_sources).map(Ok))
            .chain(undefined_repos)
            .try_collect::<Vec<_>>()
            .await
    }

    pub async fn get_repos(&self) -> io::Result<Vec<&LocalCachedRepository>> {
        try_join_all(
            error_flatten(self.get_repo_sources().await)
                .map_ok(|x| self.get_repo(x))
                .map(|x| async move {
                    match x {
                        Ok(f) => f.await,
                        Err(e) => Err(e),
                    }
                }),
        )
        .await
    }

    async fn get_repo(&self, source: RepoSource) -> io::Result<&LocalCachedRepository> {
        match source {
            RepoSource::PreDefined(source) => {
                self.repo_cache
                    .get_or_create_repo(
                        &self.get_repos_dir().joined(source.file_name),
                        source.url,
                        Some(source.name),
                    )
                    .await
            }
            RepoSource::UserRepo(user_repo) => self.repo_cache.get_user_repo(&user_repo).await,
            RepoSource::Undefined(repo_json) => {
                self.repo_cache
                    .get_repo(&repo_json, || async { unreachable!() })
                    .await
            }
        }
    }

    pub(crate) async fn find_packages(&self, package: &str) -> io::Result<Vec<PackageJson>> {
        let mut list = Vec::new();

        self.get_repos()
            .await?
            .into_iter()
            .map(|repo| repo.cache.get(package).map(Clone::clone))
            .flatten()
            .map(|x| from_value::<PackageVersions>(x).map_err(io::Error::from))
            .map_ok(|x| x.versions.into_values())
            .flatten_ok()
            .fold_ok((), |_, pkg| list.push(pkg))?;

        // user package folders
        for x in self.get_user_package_folders()? {
            if let Some(package_json) =
                load_json_or_default::<Option<PackageJson>>(&x.joined("package.json")).await?
            {
                if package_json.name == package {
                    list.push(package_json);
                }
            }
        }

        Ok(list)
    }

    pub(crate) async fn find_whole_all_packages(
        &self,
        filter: impl Fn(&PackageJson) -> bool,
    ) -> io::Result<Vec<PackageJson>> {
        let mut list = Vec::new();

        fn get_latest(versions: PackageVersions) -> Option<PackageJson> {
            versions
                .versions
                .into_values()
                .filter(|x| x.version.pre.is_empty())
                .max_by_key(|x| x.version.clone())
        }

        self.get_repos()
            .await?
            .into_iter()
            .flat_map(|repo| repo.cache.values().cloned())
            .map(|x| from_value::<PackageVersions>(x).map_err(io::Error::from))
            .filter_map_ok(get_latest)
            .filter_ok(|x| filter(x))
            .fold_ok((), |_, pkg| list.push(pkg))?;

        // user package folders
        for x in self.get_user_package_folders()? {
            if let Some(package_json) =
                load_json_or_default::<Option<PackageJson>>(&x.joined("package.json")).await?
            {
                if !package_json.version.pre.is_empty() && filter(&package_json) {
                    list.push(package_json);
                }
            }
        }

        list.sort_by_key(|x| Reverse(x.version.clone()));

        Ok(list
            .into_iter()
            .unique_by(|x| (x.name.clone(), x.version.clone()))
            .collect())
    }

    pub async fn add_package(
        &self,
        package: &PackageJson,
        target_packages_folder: &Path,
    ) -> io::Result<()> {
        log::debug!("adding package {}", package.name);
        let zip_file_name = format!("vrc-get-{}-{}.zip", &package.name, &package.version);
        let zip_path = {
            let mut building = self.global_dir.clone();
            building.push("Repos");
            building.push(&package.name);
            create_dir_all(&building).await?;
            building.push(&zip_file_name);
            building
        };
        let sha_path = zip_path.with_extension("zip.sha256");
        let dest_folder = target_packages_folder.join(&package.name);

        fn parse_hex(hex: [u8; 256 / 4]) -> Option<[u8; 256 / 8]> {
            let mut result = [0u8; 256 / 8];
            for i in 0..(256 / 8) {
                let upper = match hex[i * 2 + 0] {
                    c @ b'0'..=b'9' => c - b'0',
                    c @ b'a'..=b'f' => c - b'a' + 10,
                    c @ b'A'..=b'F' => c - b'A' + 10,
                    _ => return None,
                };
                let lower = match hex[i * 2 + 1] {
                    c @ b'0'..=b'9' => c - b'0',
                    c @ b'a'..=b'f' => c - b'a' + 10,
                    c @ b'A'..=b'F' => c - b'A' + 10,
                    _ => return None,
                };
                result[i] = upper << 4 | lower;
            }
            Some(result)
        }

        fn to_hex(data: &[u8]) -> String {
            static HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
            let mut result = vec![0u8; data.len() * 2];
            for i in 0..data.len() {
                result[i * 2 + 0] = HEX_CHARS[((data[i] >> 4) & 0xf) as usize];
                result[i * 2 + 1] = HEX_CHARS[((data[i] >> 0) & 0xf) as usize];
            }
            unsafe { String::from_utf8_unchecked(result) }
        }

        async fn try_cache(zip_path: &Path, sha_path: &Path) -> Option<File> {
            let mut cache_file = try_open_file(&zip_path).await.ok()??;
            let mut sha_file = try_open_file(&sha_path).await.ok()??;

            let mut buf = [0u8; 256 / 4];
            sha_file.read_exact(&mut buf).await.ok()?;

            let hex = parse_hex(buf)?;

            let mut sha256 = Sha256::default();
            let mut buffer = [0u8; 1024 * 4];

            // process sha256
            loop {
                match cache_file.read(&mut buffer).await {
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => return None,
                    Ok(0) => break,
                    Ok(size) => sha256.update(&buffer[0..size]),
                }
            }

            drop(buffer);

            let hash = sha256.finalize();
            let hash = &hash[..];
            if hash != &hex[..] {
                return None;
            }

            cache_file.seek(SeekFrom::Start(0)).await.ok()?;

            Some(cache_file)
        }

        let zip_file = if let Some(cache_file) = try_cache(&zip_path, &sha_path).await {
            cache_file
        } else {
            // file not found: err
            let mut cache_file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&zip_path)
                .await?;

            let mut sha256 = Sha256::default();

            let Some(http) = &self.http else {
                return Err(io::Error::new(io::ErrorKind::NotFound, "Offline mode"))
            };

            let mut stream = http
                .get(&package.url)
                .send()
                .await
                .err_mapped()?
                .error_for_status()
                .err_mapped()?
                .bytes_stream();

            while let Some(data) = stream.try_next().await.err_mapped()? {
                sha256.update(&data);
                cache_file.write_all(&data).await?;
            }

            cache_file.flush().await?;
            cache_file.seek(SeekFrom::Start(0)).await?;

            // write sha file
            let mut sha_file = File::create(&sha_path).await?;
            let hash_hex = to_hex(&sha256.finalize()[..]);
            let sha_file_content = format!("{} {}\n", hash_hex, zip_file_name);
            sha_file.write_all(sha_file_content.as_bytes()).await?;
            sha_file.flush().await?;
            drop(sha_file);

            cache_file
        };

        // remove dest folder before extract if exists
        remove_dir_all(&dest_folder).await.ok();

        // extract zip file
        let mut zip_reader = async_zip::tokio::read::seek::ZipFileReader::new(zip_file)
            .await
            .err_mapped()?;
        for i in 0..zip_reader.file().entries().len() {
            let entry = zip_reader.file().entries()[i].entry();
            let path = dest_folder.join(entry.filename());
            if !Self::check_path(Path::new(entry.filename())) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("directory traversal detected: {}", path.display()),
                )
                .into());
            }
            if entry.dir() {
                // if it's directory, just create directory
                create_dir_all(path).await?;
            } else {
                let mut reader = zip_reader.entry(i).await.err_mapped()?;
                create_dir_all(path.parent().unwrap()).await?;
                let mut dest_file = File::create(path).await?;
                tokio::io::copy(&mut reader, &mut dest_file).await?;
                dest_file.flush().await?;
            }
        }

        Ok(())
    }

    fn check_path(path: &Path) -> bool {
        for x in path.components() {
            match x {
                Component::Prefix(_) => return false,
                Component::RootDir => return false,
                Component::ParentDir => return false,
                Component::CurDir => {}
                Component::Normal(_) => {}
            }
        }
        true
    }

    pub(crate) fn get_user_repos(&self) -> serde_json::Result<Vec<UserRepoSetting>> {
        from_value::<Vec<UserRepoSetting>>(
            self.settings
                .get("userRepos")
                .cloned()
                .unwrap_or(Value::Array(vec![])),
        )
    }

    fn get_user_package_folders(&self) -> serde_json::Result<Vec<PathBuf>> {
        from_value(
            self.settings
                .get("userPackageFolders")
                .cloned()
                .unwrap_or(Value::Array(vec![])),
        )
    }

    fn add_user_repo(&mut self, repo: &UserRepoSetting) -> serde_json::Result<()> {
        self.settings
            .get_or_put_mut("userRepos", || Vec::<Value>::new())
            .as_array_mut()
            .expect("userRepos must be array")
            .push(to_value(repo)?);
        self.settings_changed = true;
        Ok(())
    }

    pub async fn add_remote_repo(
        &mut self,
        url: Url,
        name: Option<&str>,
    ) -> Result<(), AddRepositoryErr> {
        let user_repos = self.get_user_repos()?;
        if user_repos
            .iter()
            .any(|x| x.url.as_deref() == Some(url.as_ref()))
        {
            return Err(AddRepositoryErr::AlreadyAdded);
        }
        let Some(http) = &self.http else {
            return Err(AddRepositoryErr::OfflineMode);
        };
        let (remote_repo, etag) = download_remote_repository(&http, url.clone(), None)
            .await?
            .expect("logic failure: no etag");
        let local_path = self
            .get_repos_dir()
            .joined(format!("{}.json", uuid::Uuid::new_v4()));

        let repo_name = name.map(str::to_owned).or_else(|| {
            remote_repo
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });

        let mut local_cache = LocalCachedRepository::new(
            local_path.clone(),
            repo_name.clone(),
            Some(url.to_string()),
        );
        local_cache.cache = remote_repo
            .get("packages")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or(JsonMap::new());
        local_cache.repo = Some(remote_repo);

        // set etag
        if let Some(etag) = etag {
            local_cache
                .vrc_get
                .get_or_insert_with(Default::default)
                .etag = etag;
        }

        write_repo(&local_path, &local_cache).await?;

        self.add_user_repo(&UserRepoSetting::new(
            local_path.clone(),
            repo_name,
            Some(url.to_string()),
        ))?;
        Ok(())
    }

    pub async fn add_local_repo(
        &mut self,
        path: &Path,
        name: Option<&str>,
    ) -> Result<(), AddRepositoryErr> {
        let user_repos = self.get_user_repos()?;
        if user_repos.iter().any(|x| x.local_path.as_path() == path) {
            return Err(AddRepositoryErr::AlreadyAdded);
        }

        self.add_user_repo(&UserRepoSetting::new(
            path.to_owned(),
            name.map(str::to_owned),
            None,
        ))?;
        Ok(())
    }

    pub async fn remove_repo(
        &mut self,
        condition: impl Fn(&UserRepoSetting) -> bool,
    ) -> io::Result<bool> {
        let user_repos = self.get_user_repos()?;
        let mut indices = user_repos
            .iter()
            .enumerate()
            .filter(|(_, x)| condition(x))
            .collect::<Vec<_>>();
        indices.reverse();
        if indices.len() == 0 {
            return Ok(false);
        }

        let repos_json = self
            .settings
            .get_mut("userRepos")
            .and_then(Value::as_array_mut)
            .expect("userRepos");

        for (i, _) in &indices {
            repos_json.remove(*i);
        }

        join_all(indices.iter().map(|(_, x)| remove_file(&x.local_path))).await;
        self.settings_changed = true;
        Ok(true)
    }

    pub async fn save(&mut self) -> io::Result<()> {
        if !self.settings_changed {
            return Ok(());
        }

        create_dir_all(&self.global_dir).await?;
        let mut file = File::create(self.global_dir.join("settings.json")).await?;
        file.write_all(&to_json_vec(&self.settings)?).await?;
        file.flush().await?;
        self.settings_changed = false;
        Ok(())
    }
}

#[derive(Copy, Clone)]
pub struct PreDefinedRepoSource {
    file_name: &'static str,
    url: &'static str,
    name: &'static str,
}

#[derive(Clone)]
#[non_exhaustive]
pub enum RepoSource {
    PreDefined(PreDefinedRepoSource),
    UserRepo(UserRepoSetting),
    Undefined(PathBuf),
}

static OFFICIAL_REPO_SOURCE: PreDefinedRepoSource = PreDefinedRepoSource {
    file_name: "vrc-official.json",
    url: "https://packages.vrchat.com/official?download",
    name: "Official",
};

static CURATED_REPO_SOURCE: PreDefinedRepoSource = PreDefinedRepoSource {
    file_name: "vrc-curated.json",
    url: "https://packages.vrchat.com/curated?download",
    name: "Curated",
};

static DEFINED_REPO_SOURCES: &[PreDefinedRepoSource] = &[OFFICIAL_REPO_SOURCE, CURATED_REPO_SOURCE];

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
            AddRepositoryErr::AlreadyAdded => f.write_str("already newer package installed"),
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

async fn update_from_remote(client: &Client, path: &Path, repo: &mut LocalCachedRepository) {
    let Some(remote_url) = repo.creation_info.as_ref().and_then(|x| x.url.as_ref()) else {
        return;
    };

    let etag = repo.vrc_get.as_ref().map(|x| x.etag.as_str());
    match download_remote_repository(&client, remote_url, etag).await {
        Ok(None) => log::debug!("cache matched downloading {}", remote_url),
        Ok(Some((remote_repo, etag))) => {
            repo.cache = remote_repo
                .get("packages")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or(JsonMap::new());
            // set etag
            if let Some(etag) = etag {
                repo.vrc_get.get_or_insert_with(Default::default).etag = etag;
            } else {
                repo.vrc_get.as_mut().map(|x| x.etag.clear());
            }
            repo.repo = Some(remote_repo);
        }
        Err(e) => {
            log::error!("fetching remote repo '{}': {}", remote_url, e);
        }
    }

    match write_repo(path, repo).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("writing local repo '{}': {}", path.display(), e);
        }
    }
}

async fn write_repo(path: &Path, repo: &LocalCachedRepository) -> io::Result<()> {
    create_dir_all(path.parent().unwrap()).await?;
    let mut file = File::create(path).await?;
    file.write_all(&to_json_vec(repo)?).await?;
    file.flush().await?;
    Ok(())
}

// returns None if etag matches
pub(crate) async fn download_remote_repository(
    client: &Client,
    url: impl IntoUrl,
    etag: Option<&str>,
) -> io::Result<Option<(JsonMap, Option<String>)>> {
    fn map_err(err: reqwest::Error) -> io::Error {
        io::Error::new(io::ErrorKind::NotFound, err)
    }
    let mut request = client.get(url);
    if let Some(etag) = &etag {
        request = request.header("If-None-Match", etag.to_owned())
    }
    let response = request.send().await.err_mapped()?;
    let response = response.error_for_status().err_mapped()?;

    if etag.is_some() && response.status() == 304 {
        return Ok(None);
    }

    let etag = response
        .headers()
        .get("Etag")
        .and_then(|x| x.to_str().ok())
        .map(str::to_owned);

    Ok(Some((response.json().await.err_mapped()?, etag)))
}

mod vpm_manifest {
    use serde::Serialize;
    use serde_json::json;

    use super::*;

    #[derive(Debug)]
    pub(super) struct VpmManifest {
        json: JsonMap,
        dependencies: IndexMap<String, VpmDependency>,
        locked: IndexMap<String, VpmLockedDependency>,
        changed: bool,
    }

    impl VpmManifest {
        pub(super) fn new(json: JsonMap) -> serde_json::Result<Self> {
            Ok(Self {
                dependencies: from_value(
                    json.get("dependencies")
                        .cloned()
                        .unwrap_or(Value::Object(JsonMap::new())),
                )?,
                locked: from_value(
                    json.get("locked")
                        .cloned()
                        .unwrap_or(Value::Object(JsonMap::new())),
                )?,
                json,
                changed: false,
            })
        }

        pub(super) fn dependencies(&self) -> &IndexMap<String, VpmDependency> {
            &self.dependencies
        }

        pub(super) fn locked(&self) -> &IndexMap<String, VpmLockedDependency> {
            &self.locked
        }

        pub(super) fn add_dependency(&mut self, name: &str, dependency: VpmDependency) {
            // update both parsed and non-parsed
            self.add_value("dependencies", name, &dependency);
            self.dependencies.insert(name.to_string(), dependency);
        }

        pub(super) fn add_locked(&mut self, name: &str, dependency: VpmLockedDependency) {
            // update both parsed and non-parsed
            self.add_value("locked", name, &dependency);
            self.locked.insert(name.to_string(), dependency);
        }

        pub(crate) fn remove_packages(&mut self, names: &[&str]) {
            for name in names.into_iter().copied() {
                self.locked.remove(name);
                self.json
                    .get_mut("locked")
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove(name);
                self.dependencies.remove(name);
                self.json
                    .get_mut("dependencies")
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove(name);
            }
            self.changed = true;
        }

        fn add_value(&mut self, key0: &str, key1: &str, value: &impl Serialize) {
            let serialized = to_value(value).expect("serialize err");
            match self.json.get_mut(key0) {
                Some(Value::Object(obj)) => {
                    obj.insert(key1.to_string(), serialized);
                }
                _ => {
                    self.json.insert(key0.into(), json!({ key1: serialized }));
                }
            }
            self.changed = true;
        }

        pub(super) async fn save_to(&self, file: &Path) -> io::Result<()> {
            if !self.changed {
                return Ok(());
            }
            let mut file = File::create(file).await?;
            file.write_all(&to_json_vec(&self.json)?).await?;
            file.flush().await?;
            Ok(())
        }

        pub(crate) fn mark_and_sweep_packages(&mut self) -> HashSet<String> {
            // mark
            let mut required_packages = HashSet::<&str>::new();
            for x in self.dependencies.keys() {
                required_packages.insert(x);
            }

            let mut added_prev = required_packages.iter().copied().collect_vec();

            while !added_prev.is_empty() {
                let mut added = Vec::<&str>::new();

                for dep_name in added_prev
                    .into_iter()
                    .map_while(|name| self.locked.get(name))
                    .flat_map(|dep| dep.dependencies.keys())
                {
                    if required_packages.insert(dep_name) {
                        added.push(dep_name);
                    }
                }

                added_prev = added;
            }

            // sweep
            let removing_packages = self
                .locked
                .keys()
                .map(|x| x.clone())
                .filter(|x| !required_packages.contains(x.as_str()))
                .collect::<HashSet<_>>();

            for name in &removing_packages {
                self.locked.remove(name);
                self.json
                    .get_mut("locked")
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove(name);
            }

            removing_packages
        }
    }
}

#[derive(Debug)]
pub struct UnityProject {
    /// path to `Packages` folder.
    packages_dir: PathBuf,
    /// manifest.json
    manifest: VpmManifest,
    /// packages installed in the directory but not locked in vpm-manifest.json
    unlocked_packages: Vec<(String, Option<PackageJson>)>,
}

impl UnityProject {
    pub async fn find_unity_project(unity_project: Option<PathBuf>) -> io::Result<UnityProject> {
        let mut unity_found = unity_project
            .ok_or(())
            .or_else(|_| UnityProject::find_unity_project_path())?;
        unity_found.push("Packages");

        log::debug!(
            "initializing UnityProject with Packages folder {}",
            unity_found.display()
        );

        let manifest = unity_found.join("vpm-manifest.json");
        let vpm_manifest = VpmManifest::new(load_json_or_default(&manifest).await?)?;

        let mut unlocked_packages = vec![];

        let mut dir_reading = read_dir(&unity_found).await?;
        while let Some(dir_entry) = dir_reading.next_entry().await? {
            if let Some(read) = Self::try_read_unlocked_package(dir_entry, &vpm_manifest).await {
                unlocked_packages.push(read);
            }
        }

        Ok(UnityProject {
            packages_dir: unity_found,
            manifest: VpmManifest::new(load_json_or_default(&manifest).await?)?,
            unlocked_packages,
        })
    }

    async fn try_read_unlocked_package(
        dir_entry: DirEntry,
        vpm_manifest: &VpmManifest,
    ) -> Option<(String, Option<PackageJson>)> {
        let package_path = dir_entry.path();
        let name = package_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let package_json_path = package_path.join("package.json");
        let parsed = load_json_or_default::<Option<PackageJson>>(&package_json_path)
            .await
            .ok()
            .flatten();
        if let Some(parsed) = &parsed {
            if parsed.name == name && vpm_manifest.locked().contains_key(&parsed.name) {
                return None;
            }
        }
        Some((name, parsed))
    }

    fn find_unity_project_path() -> io::Result<PathBuf> {
        let mut candidate = env::current_dir()?;

        loop {
            candidate.push("Packages");
            candidate.push("vpm-manifest.json");

            if candidate.exists() {
                log::debug!("vpm-manifest.json found at {}", candidate.display());
                // if there's vpm-manifest.json, it's project path
                candidate.pop();
                candidate.pop();
                return Ok(candidate);
            }

            // replace vpm-manifest.json -> manifest.json
            candidate.pop();
            candidate.push("manifest.json");

            if candidate.exists() {
                log::debug!("manifest.json found at {}", candidate.display());
                // if there's manifest.json (which is manifest.json), it's project path
                candidate.pop();
                candidate.pop();
                return Ok(candidate);
            }

            // remove Packages/manifest.json
            candidate.pop();
            candidate.pop();

            log::debug!("Unity Project not found on {}", candidate.display());

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

pub struct AddPackageRequest {
    dependencies: Vec<(String, VpmDependency)>,
    locked: Vec<PackageJson>,
}

impl UnityProject {
    /// Add specified package to self project.
    ///
    /// If the package or newer one is already installed in dependencies, this does nothing
    /// and returns AlreadyNewerPackageInstalled err.
    ///
    /// If the package or newer one is already installed in locked list,
    /// this adds specified (not locked) version to dependencies
    pub async fn add_package(
        &mut self,
        env: &Environment,
        request: &PackageJson,
    ) -> Result<(), AddPackageErr> {
        let req = self.add_package_request(env, vec![request.clone()], true).await?;
        Ok(self.do_add_package_request(env, req).await?)
    }

    pub async fn upgrade_package(
        &mut self,
        env: &Environment,
        request: &PackageJson,
    ) -> Result<(), AddPackageErr> {
        let req = self.add_package_request(env, vec![request.clone()], false).await?;
        Ok(self.do_add_package_request(env, req).await?)
    }

    pub async fn add_package_request(
        &self,
        env: &Environment,
        mut packages: Vec<PackageJson>,
        to_dependencies: bool,
    ) -> Result<AddPackageRequest, AddPackageErr> {
        use crate::vpm::AddPackageErr::*;
        packages.retain(|pkg| {
            self.manifest.dependencies().get(&pkg.name).map(|dep| dep.version < pkg.version).unwrap_or(true)
        });

        if packages.len() == 0 {
            return Err(AlreadyNewerPackageInstalled);
        }

        // if same or newer requested package is in locked dependencies,
        // just add requested version into dependencies
        let mut dependencies = vec![];
        let mut locked = Vec::with_capacity(packages.len());

        for request in packages {
            let update = self.manifest.locked().get(&request.name).map(|dep| dep.version < request.version).unwrap_or(true);
            if update {
                locked.push(request);
            } else {
                dependencies.push((request.name, VpmDependency::new(request.version.clone())));
            }
        }

        if locked.len() == 0 {
            if to_dependencies {
                return Ok(AddPackageRequest {
                    dependencies,
                    locked: vec![],
                });
            } else {
                return Err(AlreadyNewerPackageInstalled);
            }
        }

        let packages = self.collect_adding_packages(env, locked).await?;

        // TODO: find for legacyFolders/Files here

        return Ok(AddPackageRequest { dependencies, locked: packages });
    }

    pub async fn do_add_package_request(
        &mut self,
        env: &Environment,
        request: AddPackageRequest,
    ) -> io::Result<()> {

        // first, add to dependencies
        for x in request.dependencies {
            self.manifest.add_dependency(&x.0, x.1);
        }

        self.do_add_packages_to_locked(env, &request.locked).await
    }

    async fn do_add_packages_to_locked(
        &mut self,
        env: &Environment,
        packages: &[PackageJson],
    ) -> io::Result<()> {
        // then, lock all dependencies
        for pkg in packages.iter() {
            self.manifest.add_locked(
                &pkg.name,
                VpmLockedDependency::new(
                    pkg.version.clone(),
                    pkg.vpm_dependencies.clone().unwrap_or_else(IndexMap::new),
                ),
            );
        }

        // resolve all packages
        let futures = packages
            .iter()
            .map(|x| env.add_package(x, &self.packages_dir))
            .collect::<Vec<_>>();
        try_join_all(futures).await?;

        Ok(())
    }

    /// Remove specified package from self project.
    ///
    /// This doesn't look packages not listed in vpm-maniefst.json.
    pub async fn remove(&mut self, names: &[&str]) -> Result<(), RemovePackageErr> {
        use crate::vpm::RemovePackageErr::*;

        // check for existence

        let mut repos = Vec::with_capacity(names.len());
        let mut not_founds = Vec::new();
        for name in names.into_iter().copied() {
            if let Some(x) = self.manifest.locked().get(name) {
                repos.push(x);
            } else {
                not_founds.push(name.to_owned());
            }
        }

        if !not_founds.is_empty() {
            return Err(NotInstalled(not_founds));
        }

        // check for conflicts: if some package requires some packages to be removed, it's conflict.

        let conflicts = self
            .all_dependencies()
            .filter(|(name, _)| !names.contains(&name.as_str()))
            .filter(|(_, dep)| names.into_iter().any(|x| dep.contains_key(*x)))
            .map(|(name, _)| String::from(name))
            .collect::<Vec<_>>();

        if !conflicts.is_empty() {
            return Err(ConflictsWith(conflicts));
        }

        // there's no conflicts. So do remove

        self.manifest.remove_packages(names);
        try_join_all(names.into_iter().map(|name| {
            remove_dir_all(self.packages_dir.join(name)).map(|x| match x {
                Ok(()) => Ok(()),
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e),
            })
        }))
        .await?;

        Ok(())
    }

    /// Remove specified package from self project.
    ///
    /// This doesn't look packages not listed in vpm-maniefst.json.
    pub async fn mark_and_sweep(&mut self) -> io::Result<HashSet<String>> {
        let removed_packages = self.manifest.mark_and_sweep_packages();

        try_join_all(removed_packages.iter().map(|name| {
            remove_dir_all(self.packages_dir.join(name)).map(|x| match x {
                Ok(()) => Ok(()),
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e),
            })
        }))
        .await?;

        Ok(removed_packages)
    }

    async fn collect_adding_packages(
        &self,
        env: &Environment,
        packages: Vec<PackageJson>,
    ) -> Result<Vec<PackageJson>, AddPackageErr> {
        #[derive(Default)]
        struct DependencyInfo {
            using: Option<PackageJson>,
            current: Option<Version>,
            // "" key for root dependencies
            requirements: HashMap<String, VersionRange>,
            dependencies: HashSet<String>,
        }

        impl DependencyInfo {
            fn new_dependency(version: Version) -> Self {
                let mut requirements = HashMap::new();
                requirements.insert(String::new(), VersionRange::same_or_later(version));
                DependencyInfo { 
                    using: None, 
                    current: None,
                    requirements, 
                    dependencies: HashSet::new(),
                }
            }

            fn add_range(&mut self, source: String, range: VersionRange) {
                self.requirements.insert(source, range);
            }

            fn remove_range(&mut self, source: &str) {
                self.requirements.remove(source);
            }

            pub(crate) fn set_using_info(&mut self, version: Version, dependencies: HashSet<String>) {
                self.current = Some(version);
                self.dependencies = dependencies;
            }

            pub(crate) fn set_package(&mut self, new_pkg: PackageJson) -> HashSet<String> {
                let mut dependencies = new_pkg.vpm_dependencies
                    .as_ref()
                    .map(|x| x.keys().cloned().collect())
                    .unwrap_or_default();
                
                self.current = Some(new_pkg.version.clone());
                std::mem::swap(&mut self.dependencies, &mut dependencies);
                self.using = Some(new_pkg);

                // using is save
                return dependencies
            }
        }

        let mut dependencies = HashMap::new();

        // first, add dependencies
        for (name, dep) in self.manifest.dependencies() {
            dependencies.insert(name.clone(), DependencyInfo::new_dependency(dep.version.clone()));
        }

        // then, add locked dependencies info
        for (source, locked) in self.manifest.locked() {
            dependencies.entry(source.clone()).or_default()
                .set_using_info(locked.version.clone(), locked.dependencies.keys().cloned().collect());

            for (dependency, range) in &locked.dependencies {
                dependencies.entry(dependency.clone()).or_default()
                    .add_range(source.clone(), range.clone())
            }
        }

        let mut packages = std::collections::VecDeque::from_iter(packages);

        while let Some(x) = packages.pop_front() {
            log::debug!("processing package {} version {}", x.name, x.version);
            let name = x.name.clone();
            let vpm_dependencies = x.vpm_dependencies.clone();
            let entry = dependencies.entry(x.name.clone()).or_default();
            let old_dependencies = entry.set_package(x);

            // remove previous dependencies if exists
            for dep in &old_dependencies {
                dependencies.get_mut(dep).unwrap().remove_range(dep);
            }

            // add new dependencies
            for (dependency, range) in vpm_dependencies.iter().flatten() {
                log::debug!("processing package {name}: dependency {dependency} version {range}");
                let entry = dependencies.entry(dependency.clone()).or_default();
                let mut install = true;

                if packages.iter().any(|x| &x.name == dependency && range.matches(&x.version)) {
                    // if installing version is good, no need to reinstall
                    install = false;
                    log::debug!("processing package {name}: dependency {dependency} version {range}: pending matches");
                } else {
                    // if already installed version is good, no need to reinstall
                    if let Some(version) = &entry.current {
                        if range.matches(version) {
                            log::debug!("processing package {name}: dependency {dependency} version {range}: existing matches");
                            install = false;
                        }
                    }
                }

                entry.add_range(name.clone(), range.clone());

                if install {
                    let found = env
                        .find_package_by_name(dependency, VersionSelector::Range(range))
                        .await?
                        .ok_or_else(|| AddPackageErr::DependencyNotFound {
                            dependency_name: dependency.clone(),
                        })?;

                    // remove existing if existing
                    packages.retain(|x| &x.name != dependency);
                    packages.push_back(found);
                }
            }
        }

        // finally, check for conflict.
        for (name, info) in &dependencies {
            if let Some(version) = &info.current {
                for (source, range) in &info.requirements {
                    if !range.matches(version) {
                        return Err(AddPackageErr::ConflictWithDependencies {
                            conflict: name.to_owned(),
                            dependency_name: source.clone(),
                        });
                    }
                }
            }
        }

        Ok(dependencies
            .into_values()
            .filter_map(|x| x.using)
            .collect())
    }

    pub async fn save(&mut self) -> io::Result<()> {
        self.manifest
            .save_to(&self.packages_dir.join("vpm-manifest.json"))
            .await
    }

    pub async fn resolve(&mut self, env: &Environment) -> Result<(), AddPackageErr> {
        // first, process locked dependencies
        let this = self as &Self;
        try_join_all(
            this.manifest
                .locked()
                .into_iter()
                .map(|(pkg, dep)| async move {
                    let pkg = env
                        .find_package_by_name(&pkg, VersionSelector::Specific(&dep.version))
                        .await?
                        .unwrap_or_else(|| panic!("some package in manifest.json not found: {pkg}"));
                    env.add_package(&pkg, &this.packages_dir).await?;
                    Result::<_, AddPackageErr>::Ok(())
                }),
        )
        .await?;
        // then, process dependencies of unlocked packages.
        let unlocked_dependencies = self
            .unlocked_packages
            .iter()
            .filter_map(|(_, pkg)| pkg.as_ref())
            .filter_map(|pkg| pkg.vpm_dependencies.as_ref())
            .flatten()
            .filter(|(k, _)| !self.manifest.locked().contains_key(k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .into_group_map();
        for (pkg_name, ranges) in unlocked_dependencies {
            let ranges = Vec::from_iter(&ranges);
            let pkg = env
                .find_package_by_name(&pkg_name, VersionSelector::Ranges(&ranges))
                .await?
                .expect("some dependencies of unlocked package not found");
            self.upgrade_package(env, &pkg).await?;
        }
        Ok(())
    }

    pub(crate) fn locked_packages(&self) -> &IndexMap<String, VpmLockedDependency> {
        return self.manifest.locked();
    }

    pub(crate) fn all_dependencies(
        &self,
    ) -> impl Iterator<Item = (&String, &IndexMap<String, VersionRange>)> {
        let dependencies_locked = self
            .manifest
            .locked()
            .into_iter()
            .map(|(name, dep)| (name, &dep.dependencies));

        let dependencies_unlocked = self
            .unlocked_packages
            .iter()
            .filter_map(|(_, json)| json.as_ref())
            .filter_map(|x| x.vpm_dependencies.as_ref().map(|y| (&x.name, y)))
            .map(|x| x);

        return dependencies_locked.chain(dependencies_unlocked);
    }
}

#[derive(Clone, Copy)]
pub enum VersionSelector<'a> {
    Latest,
    LatestIncluidingPrerelease,
    Specific(&'a Version),
    Range(&'a VersionRange),
    Ranges(&'a [&'a VersionRange]),
}

impl<'a> VersionSelector<'a> {
    pub fn satisfies(&self, version: &Version) -> bool {
        match self {
            VersionSelector::Latest => version.pre.is_empty(),
            VersionSelector::LatestIncluidingPrerelease => true,
            VersionSelector::Specific(finding) => &version == finding,
            VersionSelector::Range(range) => range.matches(version),
            VersionSelector::Ranges(ranges) => ranges.into_iter().all(|x| x.matches(version)),
        }
    }
}

#[derive(Debug)]
pub enum AddPackageErr {
    Io(io::Error),
    AlreadyNewerPackageInstalled,
    ConflictWithDependencies {
        /// conflicting package name
        conflict: String,
        /// the name of locked package
        dependency_name: String,
    },
    DependencyNotFound {
        dependency_name: String,
    },
    ConflictWithUnlocked,
}

impl fmt::Display for AddPackageErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddPackageErr::Io(ioerr) => fmt::Display::fmt(ioerr, f),
            AddPackageErr::AlreadyNewerPackageInstalled => {
                f.write_str("already newer package installed")
            }
            AddPackageErr::ConflictWithDependencies {
                conflict,
                dependency_name,
            } => write!(f, "{conflict} conflicts with {dependency_name}"),
            AddPackageErr::DependencyNotFound { dependency_name } => write!(
                f,
                "Package {dependency_name} (maybe dependencies of the package) not found"
            ),
            AddPackageErr::ConflictWithUnlocked => f.write_str("conflicts with unlocked packages"),
        }
    }
}

impl std::error::Error for AddPackageErr {}

impl From<io::Error> for AddPackageErr {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug)]
pub enum RemovePackageErr {
    Io(io::Error),
    NotInstalled(Vec<String>),
    ConflictsWith(Vec<String>),
}

impl fmt::Display for RemovePackageErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use RemovePackageErr::*;
        match self {
            Io(ioerr) => fmt::Display::fmt(ioerr, f),
            NotInstalled(names) => {
                f.write_str("the following packages are not installed: ")?;
                let mut iter = names.iter();
                f.write_str(iter.next().unwrap())?;
                while let Some(name) = iter.next() {
                    f.write_str(", ")?;
                    f.write_str(name)?;
                }
                Ok(())
            }
            ConflictsWith(names) => {
                f.write_str("removing packages conflicts with the following packages: ")?;
                let mut iter = names.iter();
                f.write_str(iter.next().unwrap())?;
                while let Some(name) = iter.next() {
                    f.write_str(", ")?;
                    f.write_str(name)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for RemovePackageErr {}

impl From<io::Error> for RemovePackageErr {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// open file or returns none if not exists
async fn try_open_file(path: &Path) -> io::Result<Option<File>> {
    match File::open(path).await {
        Ok(file) => Ok(Some(file)),
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

async fn load_json_or_else<T>(
    manifest_path: &Path,
    default: impl FnOnce() -> io::Result<T>,
) -> io::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    match try_open_file(manifest_path).await? {
        Some(file) => {
            let vec = read_to_vec(file).await?;
            let mut slice = vec.as_slice();
            slice = slice.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(slice);
            Ok(serde_json::from_slice(slice)?)
        }
        None => default(),
    }
}

async fn load_json_or_default<T>(manifest_path: &Path) -> io::Result<T>
where
    T: serde::de::DeserializeOwned + Default,
{
    load_json_or_else(manifest_path, || Ok(Default::default())).await
}

async fn read_to_vec(mut read: impl AsyncRead + Unpin) -> io::Result<Vec<u8>> {
    let mut vec = Vec::new();
    read.read_to_end(&mut vec).await?;
    Ok(vec)
}

fn to_json_vec<T>(value: &T) -> serde_json::Result<Vec<u8>>
where
    T: ?Sized + serde::Serialize,
{
    serde_json::to_vec_pretty(value)
}

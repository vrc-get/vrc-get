//! This module contains vpm core implementation
//!
//! This module might be a separated crate.

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr};
use std::future::ready;
use std::path::{Path, PathBuf};
use std::task::ready;
use std::task::Poll::Ready;
use std::{env, fmt, io};
use std::pin::pin;

use futures::future::{join, join_all, try_join_all};
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use indexmap::IndexMap;
use itertools::{Itertools as _};
use reqwest::{Client, IntoUrl, Url};
use serde_json::{from_value, to_value, Map, Value};
use tokio::fs::{create_dir_all, read_dir, remove_dir_all, remove_file, DirEntry, File, metadata};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};

use repo_holder::RepoHolder;
use utils::*;
use vpm_manifest::VpmManifest;

use crate::version::{Version, VersionRange};
use crate::vpm::structs::manifest::{VpmDependency, VpmLockedDependency};
use crate::vpm::structs::package::PackageJson;
use crate::vpm::structs::remote_repo::PackageVersions;
use crate::vpm::structs::repository::{LocalCachedRepository, RepositoryCache};
use crate::vpm::structs::setting::UserRepoSetting;

mod repo_holder;
pub mod structs;
mod utils;
mod add_package;

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
    // TODO: change type for user package info
    user_packages: Vec<(PathBuf, PackageJson)>,
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
            user_packages: Vec::new(),
            settings_changed: false,
        })
    }

    #[cfg(windows)]
    fn get_local_config_folder() -> PathBuf {
        use std::ffi::c_void;
        use std::ffi::OsString;
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

    pub async fn load_package_infos(&mut self) -> io::Result<()> {
        self.repo_cache.load_repos(self.get_repo_sources().await?).await?;
        self.load_user_package_infos().await?;
        Ok(())
    }

    async fn load_user_package_infos(&mut self) -> io::Result<()> {
        self.user_packages.clear();
        for x in self.get_user_package_folders()? {
            if let Some(package_json) =
                load_json_or_default::<Option<PackageJson>>(&x.join("package.json")).await?
            {
                self.user_packages.push((x, package_json));
            }
        }
        Ok(())
    }

    pub(crate) fn get_repos_dir(&self) -> PathBuf {
        self.global_dir.join("Repos")
    }

    pub fn find_package_by_name(
        &self,
        package: &str,
        version: VersionSelector,
    ) -> Option<PackageInfo> {
        let mut versions = self.find_packages(package);

        versions.retain(|x| version.satisfies(x.version()));

        versions.sort_by_key(|x| Reverse(x.version()));

        versions.into_iter().next()
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
            .map(|x| RepoSource::PreDefined(x, self.get_repos_dir().join(x.file_name)));
        let user_repo_sources = self.get_user_repos()?.into_iter().map(RepoSource::UserRepo);

        stream::iter(defined_sources.chain(user_repo_sources).map(Ok))
            .chain(undefined_repos)
            .try_collect::<Vec<_>>()
            .await
    }

    pub fn get_repos(&self) -> Vec<&LocalCachedRepository> {
        self.repo_cache.get_repos()
    }

    pub(crate) fn find_packages(&self, package: &str) -> Vec<PackageInfo> {
        let mut list = Vec::new();

        list.extend(
            self.get_repos()
            .into_iter()
            .flat_map(|repo| repo.cache.get(package))
            .flat_map(|x| x.versions.values())
            .map(PackageInfo::remote)
        );

        // user package folders
        for (path, package_json) in &self.user_packages {
            if package_json.name == package {
                list.push(PackageInfo::local(package_json, path));
            }
        }

        list
    }

    pub(crate) fn find_whole_all_packages(
        &self,
        filter: impl Fn(&PackageJson) -> bool,
    ) -> Vec<&PackageJson> {
        let mut list = Vec::new();

        fn get_latest(versions: &PackageVersions) -> Option<&PackageJson> {
            versions
                .versions
                .values()
                .filter(|x| x.version.pre.is_empty())
                .max_by_key(|x| &x.version)
        }

        self.get_repos()
            .into_iter()
            .flat_map(|repo| repo.cache.values())
            .filter_map(get_latest)
            .filter(|x| filter(x))
            .fold((), |_, pkg| list.push(pkg));

        // user package folders
        for (_, package_json) in &self.user_packages {
            if !package_json.version.pre.is_empty() && filter(package_json) {
                list.push(package_json);
            }
        }

        list.sort_by_key(|x| Reverse(&x.version));

        list
            .into_iter()
            .unique_by(|x| (&x.name, &x.version))
            .collect()
    }

    pub async fn add_package(
        &self,
        package: PackageInfo<'_>,
        target_packages_folder: &Path,
    ) -> io::Result<()> {
        add_package::add_package(
            &self.global_dir,
            self.http.as_ref(),
            package, 
            target_packages_folder,
        ).await
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
        local_cache.cache = RepositoryCache::new(remote_repo
            .get("packages")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or(JsonMap::new()))?;
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

    pub fn add_local_repo(
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
pub struct PackageInfo<'a> {
    inner: PackageInfoInner<'a>
}

#[derive(Copy, Clone)]
enum PackageInfoInner<'a> {
    Remote(&'a PackageJson),
    Local(&'a PackageJson, &'a Path),
}

impl <'a> PackageInfo<'a> {
    pub fn package_json(self) -> &'a PackageJson {
        // this match will be removed in the optimized code because package.json is exists at first
        match self.inner {
            PackageInfoInner::Remote(pkg) => pkg,
            PackageInfoInner::Local(pkg, _) => pkg,
        }
    }

    pub(crate) fn remote(json: &'a PackageJson) -> Self {
        Self { inner: PackageInfoInner::Remote(json) }
    }

    pub(crate) fn local(json: &'a PackageJson, path: &'a Path) -> Self {
        Self { inner: PackageInfoInner::Local(json, path) }
    }

    #[allow(unused)]
    pub fn is_remote(self) -> bool {
        matches!(self.inner, PackageInfoInner::Remote(_))
    }

    #[allow(unused)]
    pub fn is_local(self) -> bool {
        matches!(self.inner, PackageInfoInner::Local(_, _))
    }

    pub fn name(self) -> &'a str {
        &self.package_json().name
    }

    pub fn version(self) -> &'a Version {
        &self.package_json().version
    }

    pub fn vpm_dependencies(self) -> &'a IndexMap<String, VersionRange> {
        &self.package_json().vpm_dependencies
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
    PreDefined(PreDefinedRepoSource, PathBuf),
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
            match RepositoryCache::new(remote_repo
                .get("packages")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or(JsonMap::new())) {
                Ok(cache) => repo.cache = cache,
                Err(e) => log::error!("parsing remote repo '{}': {}", remote_url, e),
            }

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

        pub(super) fn add_dependency(&mut self, name: String, dependency: VpmDependency) {
            // update both parsed and non-parsed
            self.add_value("dependencies", &name, &dependency);
            self.dependencies.insert(name, dependency);
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
                    .filter_map(|name| self.locked.get(name))
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

            //log::debug!("removing: {removing_packages:?}");

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
    /// path to project folder.
    project_dir: PathBuf,
    /// manifest.json
    manifest: VpmManifest,
    /// packages installed in the directory but not locked in vpm-manifest.json
    unlocked_packages: Vec<(String, Option<PackageJson>)>,
}

impl UnityProject {
    pub async fn find_unity_project(unity_project: Option<PathBuf>) -> io::Result<UnityProject> {
        let unity_found = unity_project
            .ok_or(())
            .or_else(|_| UnityProject::find_unity_project_path())?;

        log::debug!(
            "initializing UnityProject with unity folder {}",
            unity_found.display()
        );

        let manifest = unity_found.join("Packages").joined("vpm-manifest.json");
        let vpm_manifest = VpmManifest::new(load_json_or_default(&manifest).await?)?;

        let mut unlocked_packages = vec![];

        let mut dir_reading = read_dir(unity_found.join("Packages")).await?;
        while let Some(dir_entry) = dir_reading.next_entry().await? {
            if let Some(read) = Self::try_read_unlocked_package(dir_entry, &vpm_manifest).await {
                unlocked_packages.push(read);
            }
        }

        Ok(UnityProject {
            project_dir: unity_found,
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

pub struct AddPackageRequest<'env> {
    dependencies: Vec<(&'env str, VpmDependency)>,
    locked: Vec<PackageInfo<'env>>,
    legacy_files: Vec<PathBuf>,
    legacy_folders: Vec<PathBuf>,
}

impl <'env> AddPackageRequest<'env> {
    pub fn locked(&self) -> &[PackageInfo<'env>] {
        &self.locked
    }

    pub fn dependencies(&self) -> &[(&'env str, VpmDependency)] {
        &self.dependencies
    }

    pub fn legacy_files(&self) -> &[PathBuf] {
        &self.legacy_files
    }

    pub fn legacy_folders(&self) -> &[PathBuf] {
        &self.legacy_folders
    }
}

impl UnityProject {
    pub async fn add_package_request<'env>(
        &self,
        env: &'env Environment,
        mut packages: Vec<PackageInfo<'env>>,
        to_dependencies: bool,
    ) -> Result<AddPackageRequest<'env>, AddPackageErr> {
        packages.retain(|pkg| {
            self.manifest.dependencies().get(pkg.name()).map(|dep| dep.version < *pkg.version()).unwrap_or(true)
        });

        // if same or newer requested package is in locked dependencies,
        // just add requested version into dependencies
        let mut dependencies = vec![];
        let mut locked = Vec::with_capacity(packages.len());

        for request in packages {
            let update = self.manifest.locked().get(request.name()).map(|dep| dep.version < *request.version()).unwrap_or(true);

            if to_dependencies {
                dependencies.push((request.name(), VpmDependency::new(request.version().clone())));
            }

            if update {
                locked.push(request);
            }
        }

        if locked.len() == 0 {
            // early return: 
            return Ok(AddPackageRequest {
                dependencies,
                locked: vec![],
                legacy_files: vec![],
                legacy_folders: vec![],
            });
        }

        let packages = self.collect_adding_packages(env, locked)?;

        let (legacy_files, legacy_folders) = self.collect_legacy_assets(&packages).await;

        return Ok(AddPackageRequest { 
            dependencies, 
            locked: packages,
            legacy_files,
            legacy_folders,
        });
    }

    async fn collect_legacy_assets(&self, packages: &[PackageInfo<'_>]) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let folders = packages.iter().flat_map(|x| &x.package_json().legacy_folders).map(|(path, guid)| (path, guid, false));
        let files = packages.iter().flat_map(|x| &x.package_json().legacy_files).map(|(path, guid)| (path, guid, true));
        let assets = folders.chain(files).collect::<Vec<_>>();

        enum LegacyInfo {
            FoundFile(PathBuf),
            FoundFolder(PathBuf),
            NotFound,
            GuidFile(GUID),
            GuidFolder(GUID),
        }
        use LegacyInfo::*;

        #[derive(Copy, Clone, Hash, Eq, PartialEq)]
        struct GUID([u8; 16]);

        fn try_parse_guid(guid: &str) -> Option<GUID> {
            Some(GUID(parse_hex_128(guid.as_bytes().try_into().ok()?)?))
        }

        fn is_guid(guid: &str) -> bool {
            guid.len() == 32 && guid
                .as_bytes()
                .iter()
                .all(|x| matches!(x, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F'))
        }

        let mut futures = pin!(assets.into_iter().map(|(path, guid, is_file)| async move {
            // some packages uses '/' as path separator.
            let path = PathBuf::from(path.replace('\\', "/"));
            // for security, deny absolute path.
            if path.is_absolute() {
                return NotFound
            }
            let path = self.project_dir.join(path);
            if metadata(&path).await.map(|x| x.is_file() == is_file).unwrap_or(false) {
                if is_file {
                    FoundFile(path)
                } else {
                    FoundFolder(path)
                }
            } else {
                if let Some(guid) = try_parse_guid(guid) {
                    if is_file {
                        GuidFile(guid)
                    } else {
                        GuidFolder(guid)
                    }
                } else {
                    NotFound
                }
            }
        }).collect::<FuturesUnordered<_>>());

        let mut found_files = HashSet::new();
        let mut found_folders = HashSet::new();
        let mut find_guids = HashMap::new();

        while let Some(info) = futures.next().await {
            match info {
                FoundFile(path) => {
                    found_files.insert(path.strip_prefix(&self.project_dir).unwrap().to_owned());
                },
                FoundFolder(path) => { 
                    found_folders.insert(path.strip_prefix(&self.project_dir).unwrap().to_owned());
                },
                NotFound => (),
                GuidFile(guid) => { find_guids.insert(guid, true); },
                GuidFolder(guid) => { find_guids.insert(guid, false); },
            }
        }

        if find_guids.len() != 0 {
            async fn get_guid(entry: DirEntry) -> Option<(GUID, bool, PathBuf)> {
                let path = entry.path();
                if path.extension() != Some(OsStr::new("meta")) || !entry.file_type().await.ok()?.is_file() {
                    return None
                }
                let mut file = BufReader::new(File::open(&path).await.ok()?);
                let mut buffer = String::new();
                while file.read_line(&mut buffer).await.ok()? != 0 {
                    let line = buffer.as_str();
                    if let Some(guid) = line.strip_prefix("guid: ") {
                        // current line should be line for guid.
                        if let Some(guid) = try_parse_guid(guid.trim()){
                            // remove .meta extension
                            let mut path = path;
                            path.set_extension("");
                            let is_file = metadata(&path).await.ok()?.is_file();
                            return Some((guid, is_file, path))
                        }
                    }

                    buffer.clear()
                }

                None
            }

            let mut stream = pin!(walk_dir([self.project_dir.join("Packages"), self.project_dir.join("Assets")]).filter_map(get_guid));

            while let Some((guid, is_file_actual, path)) = stream.next().await {
                if let Some(&is_file) = find_guids.get(&guid) {
                    if is_file_actual == is_file {
                        find_guids.remove(&guid);
                        if is_file {
                            found_files.insert(path.strip_prefix(&self.project_dir).unwrap().to_owned());
                        } else {
                            found_folders.insert(path.strip_prefix(&self.project_dir).unwrap().to_owned());
                        }
                    }
                }
            }
        }

        (found_files.into_iter().collect(), found_folders.into_iter().collect())
    }

    pub async fn do_add_package_request<'env>(
        &mut self,
        env: &'env Environment,
        request: AddPackageRequest<'env>,
    ) -> io::Result<()> {
        // first, add to dependencies
        for x in request.dependencies {
            self.manifest.add_dependency(x.0.to_owned(), x.1);
        }

        // then, do install
        self.do_add_packages_to_locked(env, &request.locked).await?;

        // finally try to remove legacy assets
        async fn remove_meta_file(path: PathBuf) {
            let mut building = path.into_os_string();
            building.push(".meta");
            let meta = PathBuf::from(building);

            if let Some(err) = tokio::fs::remove_file(&meta).await.err() {
                if !matches!(err.kind(), io::ErrorKind::NotFound) {
                    log::error!("error removing legacy asset at {}: {}", meta.display(), err);
                }
            }
        }

        async fn remove_file(path: PathBuf) {
            if let Some(err) = tokio::fs::remove_file(&path).await.err() {
                log::error!("error removing legacy asset at {}: {}", path.display(), err);
            }
            remove_meta_file(path).await;
        }

        async fn remove_folder(path: PathBuf) {
            if let Some(err) = tokio::fs::remove_dir_all(&path).await.err() {
                log::error!("error removing legacy asset at {}: {}", path.display(), err);
            }
            remove_meta_file(path).await;
        }

        join(
            join_all(request.legacy_files.into_iter().map(remove_file)),
            join_all(request.legacy_folders.into_iter().map(remove_folder)),
        ).await;
        Ok(())
    }

    async fn do_add_packages_to_locked(
        &mut self,
        env: &Environment,
        packages: &[PackageInfo<'_>],
    ) -> io::Result<()> {
        // then, lock all dependencies
        for pkg in packages.iter() {
            self.manifest.add_locked(
                &pkg.name(),
                VpmLockedDependency::new(
                    pkg.version().clone(),
                    pkg.vpm_dependencies().clone()
                ),
            );
        }

        let packages_folder = self.project_dir.join("Packages");

        // resolve all packages
        let futures = packages
            .iter()
            .map(|x| env.add_package(*x, &packages_folder))
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
            remove_dir_all(self.project_dir.join("Packages").joined(name)).map(|x| match x {
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
            remove_dir_all(self.project_dir.join("Packages").joined(name)).map(|x| match x {
                Ok(()) => Ok(()),
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e),
            })
        }))
        .await?;

        Ok(removed_packages)
    }

    fn collect_adding_packages<'env>(
        &self,
        env: &'env Environment,
        packages: Vec<PackageInfo<'env>>,
    ) -> Result<Vec<PackageInfo<'env>>, AddPackageErr> {
        #[derive(Default)]
        struct DependencyInfo<'env, 'a> {
            using: Option<PackageInfo<'env>>,
            current: Option<&'a Version>,
            // "" key for root dependencies
            requirements: HashMap<&'a str, &'a VersionRange>,
            dependencies: HashSet<&'a str>,
        }

        impl <'env, 'a> DependencyInfo<'env, 'a> where 'env: 'a {
            fn new_dependency(version_range: &'a VersionRange) -> Self {
                let mut requirements = HashMap::new();
                requirements.insert("", version_range);
                DependencyInfo { 
                    using: None, 
                    current: None,
                    requirements, 
                    dependencies: HashSet::new(),
                }
            }

            fn add_range(&mut self, source: &'a str, range: &'a VersionRange) {
                self.requirements.insert(source, range);
            }

            fn remove_range(&mut self, source: &str) {
                self.requirements.remove(source);
            }

            pub(crate) fn set_using_info(&mut self, version: &'a Version, dependencies: HashSet<&'a str>) {
                self.current = Some(version);
                self.dependencies = dependencies;
            }

            pub(crate) fn set_package(&mut self, new_pkg: PackageInfo<'env>) -> HashSet<&'a str> {
                let mut dependencies = new_pkg.vpm_dependencies()
                    .keys().map(|x| x.as_str()).collect();
                
                self.current = Some(&new_pkg.version());
                std::mem::swap(&mut self.dependencies, &mut dependencies);
                self.using = Some(new_pkg);

                // using is save
                return dependencies
            }
        }

        let mut dependencies = HashMap::<&str, _>::new();

        // first, add dependencies
        let root_dependencies = self.manifest.dependencies()
            .into_iter()
            .map(|(name, dep)| (name, VersionRange::same_or_later(dep.version.clone())))
            .collect_vec();
        for (name, range) in &root_dependencies {
            dependencies.insert(name, DependencyInfo::new_dependency(range));
        }

        // then, add locked dependencies info
        for (source, locked) in self.manifest.locked() {
            dependencies.entry(source).or_default()
                .set_using_info(&locked.version, locked.dependencies.keys().map(|x| x.as_str()).collect());

            for (dependency, range) in &locked.dependencies {
                dependencies.entry(dependency).or_default()
                    .add_range(source, range)
            }
        }

        let mut packages = std::collections::VecDeque::from_iter(packages);

        while let Some(x) = packages.pop_front() {
            log::debug!("processing package {} version {}", x.name(), x.version());
            let name = x.name();
            let vpm_dependencies = &x.vpm_dependencies();
            let entry = dependencies.entry(x.name()).or_default();
            let old_dependencies = entry.set_package(x);

            // remove previous dependencies if exists
            for dep in &old_dependencies {
                dependencies.get_mut(*dep).unwrap().remove_range(dep);
            }

            // add new dependencies
            for (dependency, range) in vpm_dependencies.iter() {
                log::debug!("processing package {name}: dependency {dependency} version {range}");
                let entry = dependencies.entry(dependency).or_default();
                let mut install = true;

                if packages.iter().any(|x| x.name() == dependency && range.matches(&x.version())) {
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

                entry.add_range(name, range);

                if install {
                    let found = env
                        .find_package_by_name(dependency, VersionSelector::Range(range))
                        .ok_or_else(|| AddPackageErr::DependencyNotFound {
                            dependency_name: dependency.clone(),
                        })?;

                    // remove existing if existing
                    packages.retain(|x| x.name() != dependency);
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
                            conflict: (*name).to_owned(),
                            dependency_name: (*source).to_owned(),
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
            .save_to(&self.project_dir.join("Packages").joined("vpm-manifest.json"))
            .await
    }

    pub async fn resolve(&mut self, env: &Environment) -> Result<(), AddPackageErr> {
        // first, process locked dependencies
        let this = self as &Self;
        let packages_folder = &this.project_dir.join("Packages");
        try_join_all(
            this.manifest
                .locked()
                .into_iter()
                .map(|(pkg, dep)| async move {
                    let pkg = env
                        .find_package_by_name(&pkg, VersionSelector::Specific(&dep.version))
                        .unwrap_or_else(|| panic!("some package in manifest.json not found: {pkg}"));
                    env.add_package(pkg, packages_folder).await?;
                    Result::<_, AddPackageErr>::Ok(())
                }),
        )
        .await?;
        // then, process dependencies of unlocked packages.
        let unlocked_dependencies = self
            .unlocked_packages
            .iter()
            .filter_map(|(_, pkg)| pkg.as_ref())
            .flat_map(|pkg| &pkg.vpm_dependencies)
            .filter(|(k, _)| !self.manifest.locked().contains_key(k.as_str()))
            .map(|(k, v)| (k, v))
            .into_group_map()
            .into_iter()
            .map(|(pkg_name, ranges)| {
                env.find_package_by_name(pkg_name, VersionSelector::Ranges(&ranges))
                    .expect("some dependencies of unlocked package not found")
            })
            .collect::<Vec<_>>();

        let req = self.add_package_request(&env, unlocked_dependencies, false).await?;

        self.do_add_package_request(&env, req).await?;

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
            .map(|x| (&x.name, &x.vpm_dependencies));

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

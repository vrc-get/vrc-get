use crate::repo_holder::RepoHolder;
use crate::repository::{RemotePackages, RemoteRepository};
use crate::structs::package::PackageJson;
use crate::structs::repo_cache::LocalCachedRepository;
use crate::structs::setting::UserRepoSetting;
use crate::utils::{JsonMapExt, PathBufExt};
use crate::version::{UnityVersion, Version, VersionRange};
use crate::{
    add_package, is_truthy, load_json_or_default, to_json_vec, unity_compatible, PackageInfo,
    PreDefinedRepoSource, RepoSource, DEFINED_REPO_SOURCES,
};
use futures::future::join_all;
use indexmap::IndexMap;
use itertools::Itertools;
use reqwest::{Client, Url};
use serde_json::{from_value, to_value, Map, Value};
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::{env, fmt, io};
use tokio::fs::{create_dir_all, remove_file, File};
use tokio::io::AsyncWriteExt;

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
    url_overrides: HashMap<PreDefinedRepoSource, Url>,
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
            http,
            settings: load_json_or_default(&folder.join("settings.json")).await?,
            global_dir: folder,
            repo_cache: RepoHolder::new(),
            user_packages: Vec::new(),
            settings_changed: false,
            url_overrides: HashMap::new(),
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

impl Environment {
    pub async fn load_package_infos(&mut self, update: bool) -> io::Result<()> {
        let http = if update { self.http.as_ref() } else { None };
        self.repo_cache
            .load_repos(http, self.get_repo_sources()?)
            .await?;
        self.update_user_repo_id();
        self.load_user_package_infos().await?;
        self.remove_id_duplication();
        Ok(())
    }

    fn update_user_repo_id(&mut self) {
        let user_repos = self.get_user_repos().unwrap();
        if user_repos.len() == 0 {
            return;
        }

        let json = self.settings.get_mut("userRepos").unwrap();

        // update id field
        for (i, mut repo) in user_repos.into_iter().enumerate() {
            let loaded = self.repo_cache.get_repo(&repo.local_path).unwrap();
            let id = loaded
                .id()
                .or(loaded.url().map(Url::as_str))
                .or(repo.url.as_ref().map(Url::as_str));
            if id != repo.id.as_deref() {
                repo.id = id.map(|x| x.to_owned());

                *json.get_mut(i).unwrap() = to_value(repo).unwrap();
                self.settings_changed = true;
            }
        }
    }

    fn remove_id_duplication(&mut self) {
        let user_repos = self.get_user_repos().unwrap();
        if user_repos.len() == 0 {
            return;
        }

        let json = self
            .settings
            .get_mut("userRepos")
            .unwrap()
            .as_array_mut()
            .unwrap();

        let mut used_ids = HashSet::new();
        let took = std::mem::take(json);
        *json = Vec::with_capacity(took.len());

        for (repo, repo_json) in user_repos.iter().zip_eq(took) {
            let mut to_add = true;
            if let Some(id) = repo.id.as_deref() {
                to_add = used_ids.insert(id);
            }
            if to_add {
                // this means new id
                json.push(repo_json)
            } else {
                // this means duplicated id: removed so mark as changed
                self.settings_changed = true;
                self.repo_cache.remove_repo(&repo.local_path);
            }
        }
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

    pub fn get_repos_dir(&self) -> PathBuf {
        self.global_dir.join("Repos")
    }

    pub fn find_package_by_name(
        &self,
        package: &str,
        package_selector: PackageSelector,
    ) -> Option<PackageInfo> {
        let mut versions = self.find_packages(package);

        versions.retain(|x| package_selector.satisfies(x));

        versions.sort_by_key(|x| Reverse(x.version()));

        versions.into_iter().next()
    }

    fn get_repo_sources(&self) -> io::Result<Vec<RepoSource>> {
        let defined_sources = DEFINED_REPO_SOURCES.into_iter().copied().map(|x| {
            RepoSource::PreDefined(
                x,
                self.url_overrides
                    .get(&x)
                    .cloned()
                    .unwrap_or_else(|| x.url()),
                self.get_repos_dir().join(x.file_name()),
            )
        });
        let user_repo_sources = self.get_user_repos()?.into_iter().map(RepoSource::UserRepo);

        Ok(defined_sources.chain(user_repo_sources).collect())
    }

    pub fn get_repos(&self) -> Vec<&LocalCachedRepository> {
        self.repo_cache.get_repos()
    }

    pub fn get_repo_with_path(
        &self,
    ) -> impl Iterator<Item = (&'_ PathBuf, &'_ LocalCachedRepository)> {
        self.repo_cache.get_repo_with_path()
    }

    pub fn find_packages(&self, package: &str) -> Vec<PackageInfo> {
        let mut list = Vec::new();

        list.extend(
            self.get_repos()
                .into_iter()
                .flat_map(|repo| repo.get_versions_of(package).map(move |pkg| (pkg, repo)))
                .map(|(pkg, repo)| PackageInfo::remote(pkg, repo)),
        );

        // user package folders
        for (path, package_json) in &self.user_packages {
            if package_json.name == package {
                list.push(PackageInfo::local(package_json, path));
            }
        }

        list
    }

    pub fn find_whole_all_packages(
        &self,
        filter: impl Fn(&PackageJson) -> bool,
    ) -> Vec<&PackageJson> {
        let mut list = Vec::new();

        fn get_latest(versions: &RemotePackages) -> Option<&PackageJson> {
            versions
                .all_versions()
                .filter(|x| !is_truthy(x.yanked.as_ref()))
                .filter(|x| x.version.pre.is_empty())
                .max_by_key(|x| &x.version)
        }

        self.get_repos()
            .into_iter()
            .flat_map(|repo| repo.get_packages())
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

        list.into_iter()
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
        )
        .await
    }

    pub fn get_user_repos(&self) -> serde_json::Result<Vec<UserRepoSetting>> {
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
        headers: IndexMap<String, String>,
    ) -> Result<(), AddRepositoryErr> {
        let user_repos = self.get_user_repos()?;
        if user_repos.iter().any(|x| x.url.as_ref() == Some(&url)) {
            return Err(AddRepositoryErr::AlreadyAdded);
        }
        let Some(http) = &self.http else {
            return Err(AddRepositoryErr::OfflineMode);
        };

        let (remote_repo, etag) = RemoteRepository::download_with_etag(&http, &url, &headers, None)
            .await?
            .expect("logic failure: no etag");
        let repo_name = name.or(remote_repo.name()).map(str::to_owned);

        let repo_id = remote_repo.id().map(str::to_owned);

        if let Some(repo_id) = repo_id.as_deref() {
            if user_repos.iter().any(|x| x.id.as_deref() == Some(repo_id)) {
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

        // [0-9a-zA-Z._-]+
        fn is_id_name_for_file(id: &str) -> bool {
            id.len() != 0
                && id.bytes().all(|b| match b {
                    b'0'..=b'9' => true,
                    b'a'..=b'z' => true,
                    b'A'..=b'Z' => true,
                    b'.' | b'_' | b'-' => true,
                    _ => false,
                })
        }

        // try id.json
        let file = match repo_id.as_deref() {
            Some(repo_id) if is_id_name_for_file(repo_id) => {
                let path = self.get_repos_dir().joined(format!("{}.json", repo_id));
                tokio::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .await
                    .ok()
                    .map(|f| (f, path))
            }
            _ => None,
        };

        // and then use
        let (mut file, local_path) = match file {
            Some(file) => file,
            None => {
                let local_path = self
                    .get_repos_dir()
                    .joined(format!("{}.json", uuid::Uuid::new_v4()));
                (File::create(&local_path).await?, local_path)
            }
        };

        file.write_all(&to_json_vec(&local_cache)?).await?;
        file.flush().await?;

        self.add_user_repo(&UserRepoSetting::new(
            local_path.clone(),
            repo_name,
            Some(url),
            repo_id,
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
            None,
        ))?;
        Ok(())
    }

    pub async fn remove_repo(
        &mut self,
        condition: impl Fn(&UserRepoSetting) -> bool,
    ) -> io::Result<usize> {
        let user_repos = self.get_user_repos()?;
        let mut indices = user_repos
            .iter()
            .enumerate()
            .filter(|(_, x)| condition(x))
            .collect::<Vec<_>>();
        indices.reverse();
        if indices.len() == 0 {
            return Ok(0);
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
        Ok(indices.len())
    }

    pub fn set_url_override(&mut self, repo: PreDefinedRepoSource, url: Url) {
        self.url_overrides.insert(repo, url);
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

#[derive(Clone, Copy)]
pub struct PackageSelector<'a> {
    project_unity: Option<UnityVersion>,
    version_selector: VersionSelector<'a>,
}

impl<'a> PackageSelector<'a> {
    pub fn specific_version(version: &'a Version) -> Self {
        Self {
            project_unity: None,
            version_selector: VersionSelector::Specific(version),
        }
    }

    pub fn latest_for(unity_version: Option<UnityVersion>, include_prerelease: bool) -> Self {
        Self {
            project_unity: unity_version,
            version_selector: if include_prerelease {
                VersionSelector::LatestIncluidingPrerelease
            } else {
                VersionSelector::Latest
            },
        }
    }

    pub fn range_for(unity_version: Option<UnityVersion>, range: &'a VersionRange) -> Self {
        Self {
            project_unity: unity_version,
            version_selector: VersionSelector::Range(range),
        }
    }

    pub fn ranges_for(unity_version: Option<UnityVersion>, ranges: &'a [&'a VersionRange]) -> Self {
        Self {
            project_unity: unity_version,
            version_selector: VersionSelector::Ranges(ranges),
        }
    }
}

impl<'a> PackageSelector<'a> {
    pub fn satisfies(&self, package: &PackageInfo) -> bool {
        match self.version_selector {
            VersionSelector::Specific(_) => {
                // if specific version is selected, ignore yank
            }
            _ => {
                // otherwise, check if yanked
                if package.is_yanked() {
                    return false;
                }
            }
        }

        if let Some(unity) = self.project_unity {
            if !unity_compatible(package, unity) {
                return false;
            }
        }

        return self.version_selector.satisfies(package.version());
    }
}

#[derive(Clone, Copy)]
enum VersionSelector<'a> {
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

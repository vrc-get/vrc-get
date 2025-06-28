use std::collections::HashSet;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use url::Url;

use crate::environment::vpm_settings::VpmSettings;
use crate::environment::vrc_get_settings::VrcGetSettings;
use crate::environment::{AddUserPackageResult, PackageCollection};
use crate::io::DefaultEnvironmentIo;
use crate::package_manifest::LooseManifest;
use crate::repository::RemoteRepository;
use crate::utils::{normalize_path, try_load_json};
use crate::{UserRepoSetting, io};

#[derive(Debug, Clone)]
pub struct Settings {
    /// parsed settings
    vpm: VpmSettings,
    vrc_get: VrcGetSettings,
}

impl Settings {
    pub async fn load(io: &DefaultEnvironmentIo) -> io::Result<Self> {
        let settings = VpmSettings::load(io).await?;
        let vrc_get_settings = VrcGetSettings::load(io).await?;

        Ok(Self {
            vpm: settings,
            vrc_get: vrc_get_settings,
        })
    }

    pub async fn save(&self, io: &DefaultEnvironmentIo) -> io::Result<()> {
        self.vpm.save(io).await?;

        Ok(())
    }
}

/// VCC Settings / Stores
impl Settings {
    pub fn show_prerelease_packages(&self) -> bool {
        self.vpm.show_prerelease_packages()
    }

    pub fn set_show_prerelease_packages(&mut self, value: bool) {
        self.vpm.set_show_prerelease_packages(value);
    }

    pub fn default_project_path(&self) -> Option<&str> {
        self.vpm.default_project_path()
    }

    pub fn set_default_project_path(&mut self, value: &str) {
        self.vpm.set_default_project_path(value);
    }

    pub fn project_backup_path(&self) -> Option<&str> {
        self.vpm.project_backup_path()
    }

    pub fn set_project_backup_path(&mut self, value: &str) {
        self.vpm.set_project_backup_path(value);
    }

    pub fn unity_hub_path(&self) -> &str {
        self.vpm.unity_hub()
    }

    pub fn set_unity_hub_path(&mut self, value: &str) {
        self.vpm.set_unity_hub(value);
    }
}

#[cfg(feature = "experimental-project-management")]
impl Settings {
    pub fn user_projects(&self) -> &[Box<str>] {
        self.vpm.user_projects()
    }

    pub fn retain_user_projects(&mut self, f: impl FnMut(&str) -> bool) -> Vec<Box<str>> {
        self.vpm.retain_user_projects(f)
    }

    pub fn add_user_project(&mut self, path: &str) {
        self.vpm.add_user_project(path);
    }

    pub fn remove_user_project(&mut self, path: &str) {
        self.vpm.remove_user_project(path);
    }

    pub fn load_from_db(&mut self, connection: &super::VccDatabaseConnection) -> io::Result<()> {
        let projects = connection.get_projects();
        let mut project_paths = projects
            .iter()
            .filter_map(|x| x.path())
            .collect::<HashSet<_>>();

        // remove removed projects
        self.vpm
            .retain_user_projects(|x| project_paths.contains(&x));

        // add new projects
        for x in self.vpm.user_projects() {
            project_paths.remove(x.as_ref());
        }

        for x in project_paths {
            self.vpm.add_user_project(x);
        }

        Ok(())
    }
}

/// VPM Settings (vrc-get extensions)
impl Settings {
    pub fn ignore_curated_repository(&self) -> bool {
        self.vrc_get.ignore_curated_repository()
    }

    pub fn ignore_official_repository(&self) -> bool {
        self.vrc_get.ignore_official_repository()
    }
}

/// User Package Managements
impl Settings {
    pub fn user_package_folders(&self) -> &[PathBuf] {
        self.vpm.user_package_folders()
    }

    pub fn remove_user_package(&mut self, pkg_path: &Path) {
        self.vpm.remove_user_package_folder(pkg_path);
    }

    pub async fn add_user_package(
        &mut self,
        pkg_path: &Path,
        io: &DefaultEnvironmentIo,
    ) -> AddUserPackageResult {
        if !pkg_path.is_absolute() {
            return AddUserPackageResult::NonAbsolute;
        }

        for x in self.vpm.user_package_folders() {
            if x == pkg_path {
                return AddUserPackageResult::AlreadyAdded;
            }
        }

        match try_load_json::<LooseManifest>(io, &pkg_path.join("package.json")).await {
            Ok(Some(LooseManifest(package_json))) => package_json,
            _ => {
                return AddUserPackageResult::BadPackage;
            }
        };

        self.vpm.add_user_package_folder(pkg_path.to_owned());

        AddUserPackageResult::Success
    }
}

/// Repository Managements
impl Settings {
    pub fn get_user_repos(&self) -> &[UserRepoSetting] {
        self.vpm.user_repos()
    }

    pub fn can_add_remote_repo(&self, url: &Url, remote_repo: &RemoteRepository) -> bool {
        let user_repos = self.get_user_repos();
        if user_repos.iter().any(|x| x.url() == Some(url)) {
            return false;
        }
        // should we check more urls?
        if !self.ignore_curated_repository()
            && url.as_str() == "https://packages.vrchat.com/curated?download"
        {
            return false;
        }
        if !self.ignore_official_repository()
            && url.as_str() == "https://packages.vrchat.com/official?download"
        {
            return false;
        }

        if let Some(repo_id) = remote_repo.id() {
            // if there is id, check if there is already repo with same id
            if user_repos.iter().any(|x| x.id() == Some(repo_id)) {
                return false;
            }
            if repo_id == "com.vrchat.repos.official" && !self.vrc_get.ignore_official_repository()
            {
                return false;
            }
            if repo_id == "com.vrchat.repos.curated" && !self.vrc_get.ignore_curated_repository() {
                return false;
            }
        }

        true
    }

    pub fn add_remote_repo(
        &mut self,
        url: &Url,
        name: Option<&str>,
        headers: IndexMap<Box<str>, Box<str>>,
        remote_repo: &RemoteRepository,
        path_buf: &Path,
    ) -> bool {
        if !self.can_add_remote_repo(url, remote_repo) {
            return false;
        }

        let repo_name = name.or(remote_repo.name()).map(Into::into);
        let repo_id = remote_repo.id().map(Into::into);

        let mut repo_setting = UserRepoSetting::new(
            path_buf.to_path_buf().into_boxed_path(),
            repo_name,
            Some(url.clone()),
            repo_id,
        );
        repo_setting.headers = headers;

        self.vpm.add_user_repo(repo_setting);
        true
    }

    pub fn add_local_repo(&mut self, path: &Path, name: Option<&str>) -> bool {
        let path = normalize_path(path);

        if self.get_user_repos().iter().any(|x| x.local_path() == path) {
            return false;
        }

        self.vpm.add_user_repo(UserRepoSetting::new(
            path.into(),
            name.map(Into::into),
            None,
            None,
        ));
        true
    }

    pub fn remove_repo(
        &mut self,
        condition: impl Fn(&UserRepoSetting) -> bool,
    ) -> Vec<UserRepoSetting> {
        self.vpm.retain_user_repos(|x| !condition(x))
    }

    // auto configurations

    /// Removes id-duplicated repositories
    ///
    /// If there are multiple repositories with the same id,
    /// this function will remove all but the first one.
    pub fn remove_id_duplication(&mut self) -> Vec<UserRepoSetting> {
        let user_repos = self.get_user_repos();
        if user_repos.is_empty() {
            return vec![];
        }

        let mut used_ids = HashSet::new();

        // retain operates in place, visiting each element exactly once in the original order.
        // s
        self.vpm.retain_user_repos(|repo| {
            let mut to_add = true;
            if let Some(id) = repo.id() {
                to_add = used_ids.insert(id.to_owned());
            }
            if to_add {
                // this means new id
                true
            } else {
                false
            }
        })
    }

    pub fn update_id(&mut self, loaded: &PackageCollection) -> bool {
        self.vpm.update_id(loaded)
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
                    query_builder.append_pair("headers[]", &format!("{header_name}:{value}"));
                }
                drop(query_builder);

                writeln!(builder, "{add_url}").unwrap();
            }
        }

        builder
    }
}

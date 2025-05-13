use crate::UserRepoSetting;
use crate::environment::PackageCollection;
use crate::io;
use crate::io::DefaultEnvironmentIo;
use crate::utils::{load_json_or_default, save_json};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

type JsonObject = Map<String, Value>;

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AsJson {
    #[serde(default)]
    path_to_unity_exe: Box<str>,
    #[serde(default)]
    path_to_unity_hub: Box<str>,
    #[serde(default)]
    user_projects: Vec<Box<str>>,
    #[serde(default)]
    unity_editors: Vec<Box<str>>,
    #[serde(default)]
    preferred_unity_editors: JsonObject,
    // In the current VCC, this path will be reset to default if it's null
    // and vrc-get prefers another path the VCC's one so keep null if not set
    #[serde(default)]
    default_project_path: Option<Box<str>>,
    #[serde(rename = "lastUIState")]
    #[serde(default)]
    last_ui_state: i64,
    #[serde(default)]
    skip_unity_auto_find: bool,
    #[serde(default)]
    user_package_folders: Vec<PathBuf>,
    #[serde(default)]
    window_size_data: JsonObject,
    #[serde(default)]
    skip_requirements: bool,
    #[serde(default)]
    last_news_update: Box<str>,
    #[serde(default)]
    allow_pii: bool,
    // In the current VCC, this path will be reset to default if it's null
    // and vrc-get prefers another path the VCC's one so keep null if not set
    #[serde(default)]
    project_backup_path: Option<Box<str>>,
    #[serde(default)]
    show_prerelease_packages: bool,
    #[serde(default)]
    track_community_repos: bool,
    #[serde(default)]
    selected_providers: u64,
    #[serde(default)]
    last_selected_project: Box<str>,
    #[serde(default)]
    user_repos: Vec<UserRepoSetting>,

    #[serde(flatten)]
    rest: JsonObject,
}

#[derive(Debug, Clone)]
pub(crate) struct VpmSettings {
    parsed: AsJson,
}

const JSON_PATH: &str = "settings.json";

impl VpmSettings {
    pub async fn load(io: &DefaultEnvironmentIo) -> io::Result<Self> {
        let parsed: AsJson = load_json_or_default(io, JSON_PATH.as_ref()).await?;

        Ok(Self { parsed })
    }

    pub(crate) fn user_repos(&self) -> &[UserRepoSetting] {
        &self.parsed.user_repos
    }

    pub(crate) fn user_package_folders(&self) -> &[PathBuf] {
        &self.parsed.user_package_folders
    }

    pub fn remove_user_package_folder(&mut self, path: &Path) {
        self.parsed.user_package_folders.retain(|x| x != path);
    }

    pub(crate) fn add_user_package_folder(&mut self, path: PathBuf) {
        self.parsed.user_package_folders.push(path);
    }

    pub(crate) fn update_id(&mut self, collection: &PackageCollection) -> bool {
        let json = &mut self.parsed;
        let mut changed = false;

        for repo in &mut json.user_repos {
            if let Some(cache) = collection.repositories.get(repo.local_path()) {
                if cache.repo.id() != repo.id() {
                    repo.id = cache.repo.id().map(|x| x.into());
                    changed = true;
                }
            }
        }

        changed
    }

    pub fn retain_user_repos(
        &mut self,
        mut f: impl FnMut(&UserRepoSetting) -> bool,
    ) -> Vec<UserRepoSetting> {
        let mut removed = Vec::new();

        // awaiting extract_if but not stable yet so use cloned method
        let json = &mut self.parsed;
        let cloned = json.user_repos.to_vec();
        json.user_repos.clear();

        for element in cloned {
            if f(&element) {
                json.user_repos.push(element);
            } else {
                removed.push(element);
            }
        }

        removed
    }

    pub(crate) fn add_user_repo(&mut self, repo: UserRepoSetting) {
        self.parsed.user_repos.push(repo);
    }

    pub(crate) fn show_prerelease_packages(&self) -> bool {
        self.parsed.show_prerelease_packages
    }

    pub(crate) fn set_show_prerelease_packages(&mut self, value: bool) {
        self.parsed.show_prerelease_packages = value;
    }

    pub(crate) fn default_project_path(&self) -> Option<&str> {
        self.parsed.default_project_path.as_deref()
    }

    pub(crate) fn set_default_project_path(&mut self, value: &str) {
        self.parsed.default_project_path = Some(value.into());
    }

    pub(crate) fn project_backup_path(&self) -> Option<&str> {
        self.parsed.project_backup_path.as_deref()
    }

    pub(crate) fn set_project_backup_path(&mut self, value: &str) {
        self.parsed.project_backup_path = Some(value.into());
    }

    pub(crate) fn unity_hub(&self) -> &str {
        &self.parsed.path_to_unity_hub
    }

    pub(crate) fn set_unity_hub(&mut self, path: &str) {
        self.parsed.path_to_unity_hub = path.into();
    }

    pub async fn save(&self, io: &DefaultEnvironmentIo) -> io::Result<()> {
        save_json(io, JSON_PATH.as_ref(), &self.parsed).await
    }
}

#[cfg(feature = "experimental-project-management")]
impl VpmSettings {
    pub(crate) fn user_projects(&self) -> &[Box<str>] {
        &self.parsed.user_projects
    }

    pub(crate) fn retain_user_projects(
        &mut self,
        mut f: impl FnMut(&str) -> bool,
    ) -> Vec<Box<str>> {
        let mut removed = Vec::new();

        // awaiting extract_if but not stable yet so use cloned method

        let cloned = self.parsed.user_projects.to_vec();
        self.parsed.user_projects.clear();

        for element in cloned {
            if f(element.as_ref()) {
                self.parsed.user_projects.push(element);
            } else {
                removed.push(element);
            }
        }

        removed
    }

    pub(crate) fn remove_user_project(&mut self, path: &str) {
        self.parsed.user_projects.retain(|x| x.as_ref() != path);
    }

    pub(crate) fn add_user_project(&mut self, path: &str) {
        self.parsed.user_projects.insert(0, path.into());
    }
}

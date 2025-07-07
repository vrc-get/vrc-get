use crate::UserRepoSetting;
use crate::environment::PackageCollection;
use crate::io;
use crate::io::DefaultEnvironmentIo;
use crate::utils::{load_json_or_default, save_json};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

type JsonObject = Map<String, Value>;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AsJson {
    #[serde(default)]
    path_to_unity_exe: Box<str>,
    #[serde(default)]
    path_to_unity_hub: Box<str>,
    // The current VPM toolchain has two places of storing user projects: `settings.json` and `vcc.litedb`.
    // Currently, `settings.json` is the single source of truth, and VCC will always copy
    // information of `settings.json` to `vcc.litedb`.
    //
    // However, it's announced that future VCC will remove copying `settings.json` to `vcc.litedb`.
    // There's no detailed documentation on how `settings.json` would be when migration removal becomes true.
    // However, we can assume the `userProjects` key will be absent from `settings.json` and `vcc.litedb` become
    // the single source of truth (opposite to current `settings.json`).
    //
    // To support reading the settings.json for both versions and writing for both versions
    // 1) vrc-get will skip copying the data from 'userProjects' to vcc.litedb if 'userProjects' is absent,
    //      for future VCC compatibility
    // 2) vrc-get will always emit 'userProjects' key even if 'userProjects' is absent.
    //    The future VCC will just remove 'userProjects' so this should not cause a problem,
    //       and older VCC will become compatible since 'userProjects' can become single source of truth
    //
    // See https://github.com/vrchat-community/creator-companion/issues/400#issuecomment-1855484391
    // See https://vcc.docs.vrchat.com/news/release-2.2.0/#important-notes-for-tool-developers
    #[serde(default, skip_serializing_if = "Option::is_none")]
    user_projects: Option<Vec<Box<str>>>,
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

impl Default for AsJson {
    fn default() -> Self {
        Self {
            path_to_unity_exe: Default::default(),
            path_to_unity_hub: Default::default(),
            user_projects: Some(vec![]),
            unity_editors: Default::default(),
            preferred_unity_editors: Default::default(),
            default_project_path: Default::default(),
            last_ui_state: Default::default(),
            skip_unity_auto_find: Default::default(),
            user_package_folders: Default::default(),
            window_size_data: Default::default(),
            skip_requirements: Default::default(),
            last_news_update: Default::default(),
            allow_pii: Default::default(),
            project_backup_path: Default::default(),
            show_prerelease_packages: Default::default(),
            track_community_repos: Default::default(),
            selected_providers: Default::default(),
            last_selected_project: Default::default(),
            user_repos: Default::default(),
            rest: Default::default(),
        }
    }
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
            if let Some(cache) = collection.repositories.get_by_path(repo.local_path())
                && cache.repo.id() != repo.id()
            {
                repo.id = cache.repo.id().map(|x| x.into());
                changed = true;
            }
        }

        changed
    }

    pub fn retain_user_repos(
        &mut self,
        mut f: impl FnMut(&UserRepoSetting) -> bool,
    ) -> Vec<UserRepoSetting> {
        self.parsed
            .user_repos
            .extract_if(.., |r| !f(r))
            .collect::<Vec<_>>()
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
    pub(crate) fn user_projects(&self) -> Option<&[Box<str>]> {
        self.parsed.user_projects.as_deref()
    }

    pub(crate) fn retain_user_projects(
        &mut self,
        mut f: impl FnMut(&str) -> bool,
    ) -> Option<Vec<Box<str>>> {
        Some(
            (self.parsed.user_projects.as_mut())?
                .extract_if(.., |x| !f(x))
                .collect(),
        )
    }

    pub(crate) fn remove_user_project(&mut self, path: &str) {
        if let Some(x) = self.parsed.user_projects.as_mut() {
            x.retain(|x| x.as_ref() != path)
        }
    }

    pub(crate) fn add_user_project(&mut self, path: &str) {
        self.parsed
            .user_projects
            .get_or_insert_default()
            .insert(0, path.into());
    }
}

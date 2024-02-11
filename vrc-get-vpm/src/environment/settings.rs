use crate::io;
use crate::io::EnvironmentIo;
use crate::utils::{load_json_or_default, SaveController};
use crate::UserRepoSetting;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::PathBuf;

type JsonObject = Map<String, Value>;

#[derive(Debug, Default, Serialize, Deserialize)]
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
    #[serde(default)]
    default_project_path: Box<str>,
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
    #[serde(default)]
    project_backup_path: Box<str>,
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

#[derive(Debug)]
pub(crate) struct Settings {
    controller: SaveController<AsJson>,
}

pub(crate) trait NewIdGetter {
    // I wanted to be closure but it looks not possible
    // https://users.rust-lang.org/t/any-way-to-return-an-closure-that-would-returns-a-reference-to-one-of-its-captured-variable/22652/2
    fn new_id<'a>(&'a self, repo: &'a UserRepoSetting) -> Option<&'a str>;
}

const JSON_PATH: &str = "settings.json";

impl Settings {
    pub async fn load(io: &impl EnvironmentIo) -> io::Result<Self> {
        let parsed = load_json_or_default(io, JSON_PATH.as_ref()).await?;

        Ok(Self {
            controller: SaveController::new(parsed),
        })
    }

    pub(crate) fn user_repos(&self) -> &[UserRepoSetting] {
        &self.controller.user_repos
    }

    pub(crate) fn user_package_folders(&self) -> &[PathBuf] {
        &self.controller.user_package_folders
    }

    pub(crate) fn update_user_repo_id(&mut self, new_id: impl NewIdGetter) {
        self.controller.may_changing(|json| {
            let mut changed = false;
            for repo in &mut json.user_repos {
                let id = new_id.new_id(repo);
                if id != repo.id() {
                    let owned = id.map(|x| x.into());
                    repo.id = owned;
                    changed = true;
                }
            }
            changed
        })
    }

    pub fn retain_user_repos(
        &mut self,
        mut f: impl FnMut(&UserRepoSetting) -> bool,
    ) -> Vec<UserRepoSetting> {
        let mut removed = Vec::new();

        // awaiting extract_if but not stable yet so use cloned method
        self.controller.may_changing(|json| {
            let cloned = json.user_repos.to_vec();
            json.user_repos.clear();

            for element in cloned {
                if f(&element) {
                    json.user_repos.push(element);
                } else {
                    removed.push(element);
                }
            }

            !removed.is_empty()
        });

        removed
    }

    pub(crate) fn add_user_repo(&mut self, repo: UserRepoSetting) {
        self.controller.as_mut().user_repos.push(repo);
    }

    pub async fn save(&mut self, io: &impl EnvironmentIo) -> io::Result<()> {
        self.controller.save(io, JSON_PATH.as_ref()).await
    }
}

#[cfg(feature = "experimental-project-management")]
impl Settings {
    pub(crate) fn user_projects(&self) -> &[Box<str>] {
        &self.controller.user_projects
    }

    pub(crate) fn remove_user_project(&mut self, path: &str) {
        self.controller
            .as_mut()
            .user_projects
            .retain(|x| x.as_ref() != path);
    }

    pub(crate) fn add_user_project(&mut self, path: &str) {
        self.controller
            .as_mut()
            .user_projects
            .insert(0, path.into());
    }
}

#[cfg(feature = "experimental-unity-management")]
impl Settings {
    pub(crate) fn unity_hub(&self) -> &str {
        &self.controller.path_to_unity_hub
    }

    pub(crate) fn set_unity_hub(&mut self, path: &str) {
        self.controller.as_mut().path_to_unity_hub = path.into();
    }
}

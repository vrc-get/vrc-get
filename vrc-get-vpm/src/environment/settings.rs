use crate::io::EnvironmentIo;
use crate::utils::{load_json_or_default2, to_vec_pretty_os_eol};
use crate::UserRepoSetting;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::io;
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
    as_json: AsJson,

    settings_changed: bool,
}

pub(crate) trait NewIdGetter {
    // I wanted to be closure but it looks not possible
    // https://users.rust-lang.org/t/any-way-to-return-an-closure-that-would-returns-a-reference-to-one-of-its-captured-variable/22652/2
    fn new_id<'a>(&'a self, repo: &'a UserRepoSetting) -> Option<&'a str>;
}

const JSON_PATH: &str = "settings.json";

impl Settings {
    pub async fn load(io: &impl EnvironmentIo) -> io::Result<Self> {
        let parsed = load_json_or_default2(io, JSON_PATH.as_ref()).await?;

        Ok(Self {
            as_json: parsed,
            settings_changed: false,
        })
    }

    pub(crate) fn user_repos(&self) -> &[UserRepoSetting] {
        &self.as_json.user_repos
    }

    pub(crate) fn user_package_folders(&self) -> &[PathBuf] {
        &self.as_json.user_package_folders
    }

    pub(crate) fn update_user_repo_id(&mut self, new_id: impl NewIdGetter) {
        for repo in &mut self.as_json.user_repos {
            let id = new_id.new_id(repo);
            if id != repo.id() {
                let owned = id.map(|x| x.into());
                repo.id = owned;
                self.settings_changed = true;
            }
        }
    }

    pub fn retain_user_repos(
        &mut self,
        mut f: impl FnMut(&UserRepoSetting) -> bool,
    ) -> Vec<UserRepoSetting> {
        // awaiting extract_if but not stable yet so use cloned method
        let cloned = self.as_json.user_repos.to_vec();
        self.as_json.user_repos.clear();
        let mut removed = Vec::new();

        for element in cloned {
            if f(&element) {
                self.as_json.user_repos.push(element);
            } else {
                removed.push(element);
            }
        }

        removed
    }

    pub(crate) fn add_user_repo(&mut self, repo: UserRepoSetting) {
        self.as_json.user_repos.push(repo);
        self.settings_changed = true;
    }

    pub async fn save(&mut self, io: &impl EnvironmentIo) -> io::Result<()> {
        if !self.settings_changed {
            return Ok(());
        }

        io.create_dir_all(".").await?;
        io.write(JSON_PATH, &to_vec_pretty_os_eol(&self.as_json)?)
            .await?;

        self.settings_changed = false;
        Ok(())
    }
}

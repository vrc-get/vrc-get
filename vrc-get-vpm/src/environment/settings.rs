use crate::{load_json_or_default, to_json_vec, UserRepoSetting};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::io;
use std::path::PathBuf;
use tokio::fs::create_dir_all;

type JsonObject = Map<String, Value>;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AsJson {
    #[serde(default)]
    path_to_unity_exe: String,
    #[serde(default)]
    path_to_unity_hub: String,
    #[serde(default)]
    user_projects: Vec<String>,
    #[serde(default)]
    unity_editors: Vec<String>,
    #[serde(default)]
    preferred_unity_editors: JsonObject,
    #[serde(default)]
    default_project_path: String,
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
    last_news_update: String,
    #[serde(default)]
    allow_pii: bool,
    #[serde(default)]
    project_backup_path: String,
    #[serde(default)]
    show_prerelease_packages: bool,
    #[serde(default)]
    track_community_repos: bool,
    #[serde(default)]
    selected_providers: u64,
    #[serde(default)]
    last_selected_project: String,
    #[serde(default)]
    user_repos: Vec<UserRepoSetting>,

    #[serde(flatten)]
    rest: Map<String, Value>,
}

#[derive(Debug)]
pub(crate) struct Settings {
    as_json: AsJson,

    path: PathBuf,

    settings_changed: bool,
}

pub(crate) trait NewIdGetter {
    // I wanted to be closure but it looks not possible
    // https://users.rust-lang.org/t/any-way-to-return-an-closure-that-would-returns-a-reference-to-one-of-its-captured-variable/22652/2
    fn new_id<'a>(&'a self, repo: &'a UserRepoSetting) -> Option<&'a str>;
}

impl Settings {
    pub async fn load(json_path: PathBuf) -> io::Result<Self> {
        let parsed = load_json_or_default(&json_path).await?;

        Ok(Self {
            as_json: parsed,
            path: json_path,
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
                let owned = id.map(|x| x.to_owned());
                repo.id = owned;
                self.settings_changed = true;
            }
        }
    }

    pub fn retain_user_repos(
        &mut self,
        f: impl FnMut(&UserRepoSetting) -> bool,
    ) -> usize {
        let prev_count = self.as_json.user_repos.len();
        self.as_json.user_repos.retain(f);
        let new_count = self.as_json.user_repos.len();

        if prev_count != new_count {
            self.settings_changed = true;
        }

        prev_count - new_count
    }

    pub(crate) fn add_user_repo(&mut self, repo: UserRepoSetting) {
        self.as_json.user_repos.push(repo);
        self.settings_changed = true;
    }

    pub async fn save(&mut self) -> io::Result<()> {
        if !self.settings_changed {
            return Ok(());
        }

        let json_path = &self.path;

        if let Some(parent) = json_path.parent() {
            create_dir_all(&parent).await?;
        }

        tokio::fs::write(json_path, &to_json_vec(&self.as_json)?).await?;
        self.settings_changed = false;
        Ok(())
    }
}

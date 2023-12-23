use crate::{to_json_vec, UserRepoSetting};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::io;
use std::path::{Path, PathBuf};
use tokio::fs::{create_dir_all, File};
use tokio::io::AsyncWriteExt;

type JsonObject = Map<String, Value>;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Settings {
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

    #[serde(skip)]
    settings_changed: bool,
}

pub(crate) trait NewIdGetter {
    // I wanted to be closure but it looks not possible
    // https://users.rust-lang.org/t/any-way-to-return-an-closure-that-would-returns-a-reference-to-one-of-its-captured-variable/22652/2
    fn new_id<'a>(&'a self, repo: &'a UserRepoSetting) -> Option<&'a str>;
}

impl Settings {
    pub(crate) fn user_repos(&self) -> &[UserRepoSetting] {
        &self.user_repos
    }

    pub(crate) fn user_package_folders(&self) -> &[PathBuf] {
        &self.user_package_folders
    }

    pub(crate) fn update_user_repo_id(&mut self, new_id: impl NewIdGetter) {
        for repo in &mut self.user_repos {
            let id = new_id.new_id(repo);
            if id != repo.id() {
                let owned = id.map(|x| x.to_owned());
                repo.id = owned;
                self.settings_changed = true;
            }
        }
    }

    pub(crate) fn retain_user_repos(
        &mut self,
        f: impl FnMut(&UserRepoSetting) -> bool,
    ) -> usize {
        let prev_count = self.user_repos.len();
        self.user_repos.retain(f);
        let new_count = self.user_repos.len();

        if prev_count != new_count {
            self.settings_changed = true;
        }

        prev_count - new_count
    }

    pub(crate) fn add_user_repo(&mut self, repo: UserRepoSetting) {
        self.user_repos.push(repo);
        self.settings_changed = true;
    }

    pub fn changed(&self) -> bool {
        self.settings_changed
    }

    pub(crate) async fn save_to(&mut self, json_path: &Path) -> io::Result<()> {
        if let Some(parent) = json_path.parent() {
            create_dir_all(&parent).await?;
        }

        let mut file = File::create(json_path).await?;
        file.write_all(&to_json_vec(&self)?).await?;
        file.flush().await?;
        self.settings_changed = false;
        Ok(())
    }
}

use crate::UserRepoSetting;
use crate::environment::PackageCollection;
use crate::io;
use crate::io::DefaultEnvironmentIo;
use crate::utils::json::{JsonArray, JsonError, JsonObject, JsonValue, save_json, try_load_json};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)]
struct AsJson {
    path_to_unity_exe: Box<str>,
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
    user_projects: Option<Vec<Box<str>>>,
    unity_editors: Vec<Box<str>>,
    preferred_unity_editors: JsonValue,
    // In the current VCC, this path will be reset to default if it's null
    // and vrc-get prefers another path the VCC's one so keep null if not set
    default_project_path: Option<Box<str>>,
    last_ui_state: JsonValue,
    skip_unity_auto_find: JsonValue,
    user_package_folders: Vec<PathBuf>,
    window_size_data: JsonValue,
    skip_requirements: JsonValue,
    last_news_update: JsonValue,
    allow_pii: JsonValue,
    // In the current VCC, this path will be reset to default if it's null
    // and vrc-get prefers another path the VCC's one so keep null if not set
    project_backup_path: Option<Box<str>>,
    show_prerelease_packages: bool,
    track_community_repos: JsonValue,
    selected_providers: JsonValue,
    last_selected_project: JsonValue,
    user_repos: Vec<UserRepoSetting>,
    raw: JsonObject,
}

impl AsJson {
    fn from_json_value(value: JsonValue) -> Result<Self, JsonError> {
        let object = value.into_object()?;
        Ok(Self {
            path_to_unity_exe: (object.get_opt("pathToUnityExe"))
                .try_map(JsonValue::into_string)?
                .unwrap_or(String::new())
                .into(),
            path_to_unity_hub: (object.get_opt("pathToUnityHub"))
                .try_map(JsonValue::into_string)?
                .unwrap_or(String::new())
                .into(),
            user_projects: (object.get_opt("userProjects")).try_map(|value| {
                value.into_array().and_then(|array| {
                    array
                        .into_iter()
                        .map(|x| x.into_string().map(Into::into))
                        .collect::<Result<Vec<_>, _>>()
                })
            })?,
            unity_editors: (object.get_opt("unityEditors"))
                .try_map(|value| {
                    value.into_array().and_then(|array| {
                        array
                            .into_iter()
                            .map(|x| x.into_string().map(Into::into))
                            .collect::<Result<Vec<_>, _>>()
                    })
                })?
                .unwrap_or(vec![]),
            preferred_unity_editors: object.get_opt("preferredUnityEditors"),
            default_project_path: (object.get_opt("defaultProjectPath"))
                .try_map(JsonValue::into_string)?
                .map(Into::into),
            last_ui_state: object.get_opt("lastUIState"),
            skip_unity_auto_find: object.get_opt("skipUnityAutoFind"),
            user_package_folders: (object.get_opt("userPackageFolders"))
                .try_map(|value| {
                    value.into_array().and_then(|array| {
                        array
                            .into_iter()
                            .map(|x| x.into_string().map(PathBuf::from))
                            .collect::<Result<Vec<_>, _>>()
                    })
                })?
                .unwrap_or(vec![]),
            window_size_data: object.get_opt("windowSizeData"),
            skip_requirements: object.get_opt("skipRequirements"),
            last_news_update: object.get_opt("lastNewsUpdate"),
            allow_pii: object.get_opt("allowPii"),
            project_backup_path: (object.get_opt("projectBackupPath"))
                .try_map(JsonValue::into_string)?
                .map(Into::into),
            show_prerelease_packages: (object.get_opt("showPrereleasePackages"))
                .try_map(JsonValue::into_bool)?
                .unwrap_or(false),
            track_community_repos: object.get_opt("trackCommunityRepos"),
            selected_providers: object.get_opt("selectedProviders"),
            last_selected_project: object.get_opt("lastSelectedProject"),
            user_repos: (object.get_opt("userRepos"))
                .try_map(|value| {
                    value.into_array().and_then(|array| {
                        array
                            .into_iter()
                            .map(UserRepoSetting::from_json_value)
                            .collect::<Result<Vec<_>, _>>()
                    })
                })?
                .unwrap_or(vec![]),
            raw: object,
        })
    }

    fn to_json_value(&self) -> JsonValue {
        let mut object = self.raw.clone();
        object.insert("pathToUnityExe", &self.path_to_unity_exe);
        object.insert("pathToUnityHub", &self.path_to_unity_hub);
        if let Some(user_projects) = &self.user_projects {
            object.insert("userProjects", user_projects.as_slice());
        } else {
            object.remove("userProjects");
        }
        object.insert("unityEditors", self.unity_editors.as_slice());
        object.insert("preferredUnityEditors", &self.preferred_unity_editors);
        object.insert("defaultProjectPath", self.default_project_path.as_deref());
        object.insert("lastUIState", &self.last_ui_state);
        object.insert("skipUnityAutoFind", &self.skip_unity_auto_find);
        object.insert(
            "userPackageFolders",
            self.user_package_folders
                .iter()
                .map(|x| x.to_string_lossy())
                .collect::<JsonArray>(),
        );
        object.insert("windowSizeData", &self.window_size_data);
        object.insert("skipRequirements", &self.skip_requirements);
        object.insert("lastNewsUpdate", &self.last_news_update);
        object.insert("allowPii", self.allow_pii.clone());
        object.insert("projectBackupPath", self.project_backup_path.as_deref());
        object.insert("showPrereleasePackages", self.show_prerelease_packages);
        object.insert("trackCommunityRepos", self.track_community_repos.clone());
        object.insert("selectedProviders", &self.selected_providers);
        object.insert("lastSelectedProject", &self.last_selected_project);
        object.insert(
            "userRepos",
            self.user_repos
                .iter()
                .map(|r| r.to_json_value())
                .collect::<JsonArray>(),
        );
        object.into()
    }
}

#[derive(Default, Debug, Clone)]
pub(crate) struct VpmSettings {
    parsed: AsJson,
}

const JSON_PATH: &str = "settings.json";
const ALT_JSON_PATH: &str = "vrc-get/vcc-settings-backup.json";

impl VpmSettings {
    pub async fn load(io: &DefaultEnvironmentIo) -> io::Result<Option<Self>> {
        Self::load_inner(io, JSON_PATH).await
    }

    pub async fn load_alt(io: &DefaultEnvironmentIo) -> io::Result<Option<Self>> {
        let mut settings = Self::load_inner(io, ALT_JSON_PATH).await?;

        // We use data from vcc.litedb for the source of the projecs list since it's much reliable source.
        if let Some(ref mut settings) = settings {
            settings.parsed.user_projects = None;
        }

        Ok(settings)
    }

    async fn load_inner(io: &DefaultEnvironmentIo, path: &str) -> io::Result<Option<Self>> {
        let Some(parsed) = try_load_json(io, path.as_ref(), AsJson::from_json_value).await? else {
            log::debug!("VpmSettings Configuration file not found at {path}");
            return Ok(None);
        };

        log::debug!("Parsed VpmSettings at {path}");

        Ok(Some(Self { parsed }))
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

    pub fn remove_user_repo_at_index(&mut self, index: usize) -> Option<UserRepoSetting> {
        let repos = &mut self.parsed.user_repos;
        if index < repos.len() {
            Some(repos.remove(index))
        } else {
            None
        }
    }

    pub fn reorder_user_repos_by_indices(&mut self, indices: &[usize]) {
        let mut pool: Vec<Option<UserRepoSetting>> = std::mem::take(&mut self.parsed.user_repos)
            .into_iter()
            .map(Some)
            .collect();
        let mut result = Vec::with_capacity(pool.len());
        for &idx in indices {
            if let Some(slot) = pool.get_mut(idx)
                && let Some(repo) = slot.take()
            {
                result.push(repo);
            }
        }
        result.extend(pool.into_iter().flatten());
        self.parsed.user_repos = result;
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
        let value = self.parsed.to_json_value();
        save_json(io, JSON_PATH.as_ref(), &value).await?;
        save_json(io, ALT_JSON_PATH.as_ref(), &value).await
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

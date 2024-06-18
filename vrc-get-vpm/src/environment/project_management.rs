use crate::io::{EnvironmentIo, FileSystemProjectIo, ProjectIo};
use crate::utils::{check_absolute_path, normalize_path, PathBufExt};
use crate::version::UnityVersion;
use crate::{io, Environment, HttpClient, ProjectType, UnityProject};
use bson::oid::ObjectId;
use bson::DateTime;
use futures::future::join_all;
use log::error;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

pub(crate) static COLLECTION: &str = "projects";

impl<T: HttpClient, IO: EnvironmentIo> Environment<T, IO> {
    pub async fn migrate_from_settings_json(&mut self) -> io::Result<()> {
        // remove relative paths
        let removed = self
            .settings
            .retain_user_projects(|x| Path::new(x).is_absolute());
        if !removed.is_empty() {
            error!("Removed relative paths: {:?}", removed);
        }

        let db = self.get_db()?; // ensure the database connection is initialized

        let projects = self
            .settings
            .user_projects()
            .iter()
            .map(|x| x.as_ref())
            .collect::<HashSet<_>>();

        let db_projects = db
            .get_values::<UserProject>(COLLECTION)?
            .into_iter()
            .map(|x| (x.path().to_owned(), x))
            .collect::<std::collections::HashMap<_, _>>();

        // add new projects
        for project in &projects {
            if !db_projects.contains_key(*project) {
                async fn get_project_type(
                    io: &impl EnvironmentIo,
                    path: &Path,
                ) -> io::Result<(ProjectType, Option<UnityVersion>, Option<String>)>
                {
                    let project = UnityProject::load(io.new_project_io(path)).await?;
                    let detected_type = project.detect_project_type().await?;
                    Ok((
                        detected_type,
                        project.unity_version(),
                        project.unity_revision().map(|x| x.to_owned()),
                    ))
                }
                let (project_type, unity_version, unity_revision) =
                    get_project_type(&self.io, project.as_ref())
                        .await
                        .unwrap_or((ProjectType::Unknown, None, None));
                let mut project = UserProject::new((*project).into(), unity_version, project_type);
                if let (Some(unity), Some(revision)) = (unity_version, unity_revision) {
                    project.set_unity_revision(unity, revision);
                }
                db.insert(COLLECTION, &project)?;
            }
        }

        // remove deleted projects
        for (project_path, project) in db_projects.iter() {
            if !projects.contains(project_path.as_str()) {
                db.delete(COLLECTION, project.id)?;
            }
        }

        Ok(())
    }

    pub async fn sync_with_real_projects(&mut self, skip_not_found: bool) -> io::Result<()> {
        let db = self.get_db()?; // ensure the database connection is initialized

        let mut projects = db.get_values::<UserProject>(COLLECTION)?;

        let changed_projects = join_all(
            projects
                .iter_mut()
                .map(|x| update_project_with_actual_data(&self.io, x, skip_not_found)),
        )
        .await;

        for project in changed_projects.iter().flatten() {
            db.update(COLLECTION, project)?;
        }

        async fn update_project_with_actual_data<'a>(
            io: &impl EnvironmentIo,
            project: &'a mut UserProject,
            skip_not_found: bool,
        ) -> Option<&'a UserProject> {
            match update_project_with_actual_data_inner(io, project, skip_not_found).await {
                Ok(Some(project)) => Some(project),
                Ok(None) => None,
                Err(err) => {
                    error!("Error updating project information: {}", err);
                    None
                }
            }
        }

        async fn update_project_with_actual_data_inner<'a>(
            io: &impl EnvironmentIo,
            project: &'a mut UserProject,
            skip_not_found: bool,
        ) -> io::Result<Option<&'a UserProject>> {
            let path = project.path().as_ref();

            if !io.is_dir(path).await {
                if !skip_not_found {
                    error!("Project {} not found", path.display());
                }
                return Ok(None);
            }

            let normalized = normalize_path(path);
            let normalized = if normalized != path {
                Some(normalized)
            } else {
                None
            };

            let mut changed = false;

            let loaded_project = UnityProject::load(io.new_project_io(path)).await?;
            if let Some(unity_version) = loaded_project.unity_version() {
                if let Some(revision) = loaded_project.unity_revision() {
                    if Some(unity_version) != project.unity_version()
                        || Some(revision) != project.unity_revision()
                    {
                        changed = true;
                        project.set_unity_revision(unity_version, revision.to_owned());
                    }
                } else {
                    #[allow(clippy::collapsible_else_if)]
                    if project.unity_version() != Some(unity_version) {
                        changed = true;
                        project.set_unity_version(unity_version);
                    }
                }
            }

            let project_type = loaded_project.detect_project_type().await?;
            if project.project_type() != project_type {
                changed = true;
                project.project_type = project_type;
            }

            if let Some(normalized) = normalized {
                changed = true;
                project.path = normalized.to_str().unwrap().into();
            }

            Ok(if changed { Some(project) } else { None })
        }

        Ok(())
    }

    pub fn dedup_projects(&mut self) -> io::Result<()> {
        let db = self.get_db()?; // ensure the database connection is initialized

        let projects = db.get_values::<UserProject>(COLLECTION)?;

        let mut projects_by_path = HashMap::<_, Vec<_>>::new();

        for project in &projects {
            projects_by_path
                .entry(project.path())
                .or_default()
                .push(project);
        }

        for (_, mut values) in projects_by_path {
            if values.len() == 1 {
                continue;
            }

            // update favorite and last modified

            let favorite = values.iter().any(|x| x.favorite());
            let last_modified = values.iter().map(|x| x.last_modified()).max().unwrap();

            let mut project = values[0].clone();
            let mut changed = false;
            if project.favorite() != favorite {
                project.set_favorite(favorite);
                changed = true;
            }
            if project.last_modified() != last_modified {
                project.last_modified = last_modified;
                changed = true;
            }

            if changed {
                db.update(COLLECTION, &project)?;
            }

            // remove rest
            for project in values.iter().skip(1) {
                db.delete(COLLECTION, project.id)?;
            }
        }

        Ok(())
    }

    // TODO: return wrapper type instead?
    pub fn get_projects(&self) -> io::Result<Vec<UserProject>> {
        Ok(self.get_db()?.get_values(COLLECTION)?)
    }

    pub fn find_project(&self, project_path: &Path) -> io::Result<Option<UserProject>> {
        check_absolute_path(project_path)?;
        let db = self.get_db()?;
        let project_path = normalize_path(project_path);

        let mut project = db.get_values::<UserProject>(COLLECTION)?;
        Ok(project
            .into_iter()
            .find(|x| Path::new(x.path()) == project_path))
    }

    pub fn update_project_last_modified(&mut self, project_path: &Path) -> io::Result<()> {
        check_absolute_path(project_path)?;
        let Some(mut project) = self.find_project(project_path)? else {
            return Ok(());
        };

        project.last_modified = DateTime::now();
        self.update_project(&project)?;
        Ok(())
    }

    pub fn update_project(&mut self, project: &UserProject) -> io::Result<()> {
        Ok(self.get_db()?.update(COLLECTION, &project)?)
    }

    pub fn remove_project(&mut self, project: &UserProject) -> io::Result<()> {
        let db = self.get_db()?;

        db.delete(COLLECTION, project.id)?;
        self.settings.remove_user_project(project.path());

        Ok(())
    }

    pub async fn add_project<ProjectIO: ProjectIo + FileSystemProjectIo>(
        &mut self,
        project: &UnityProject<ProjectIO>,
    ) -> io::Result<()> {
        check_absolute_path(project.project_dir())?;
        let path = normalize_path(project.project_dir());
        let path = path.to_str().ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "project path is not utf8",
        ))?;
        let unity_version = project.unity_version().ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "project has no unity version",
        ))?;
        let unity_revision = project.unity_revision().ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "project has no unity revision",
        ))?;

        let project_type = project.detect_project_type().await?;

        let mut new_project = UserProject::new(path.into(), Some(unity_version), project_type);
        new_project.set_unity_revision(unity_version, unity_revision.to_owned());

        self.get_db()?.insert(COLLECTION, &new_project)?;
        self.settings.add_user_project(path);

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserProject {
    #[serde(rename = "_id")]
    id: ObjectId,
    #[serde(rename = "Path")]
    path: Box<str>,
    #[serde(default, rename = "UnityVersion")]
    unity_version: Option<UnityVersion>,
    #[serde(rename = "CreatedAt")]
    created_at: DateTime,
    #[serde(rename = "LastModified")]
    last_modified: DateTime,
    #[serde(rename = "Type")]
    project_type: ProjectType,
    #[serde(rename = "Favorite")]
    favorite: bool,
    #[serde(default, rename = "vrc-get")]
    vrc_get: Option<VrcGetMeta>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct VrcGetMeta {
    #[serde(default)]
    cached_unity_version: Option<UnityVersion>,
    #[serde(default)]
    unity_revision: Option<String>,
    custom_unity_args: Option<Vec<String>>,
}

impl UserProject {
    fn new(path: Box<str>, unity_version: Option<UnityVersion>, project_type: ProjectType) -> Self {
        let now = DateTime::now();
        Self {
            id: ObjectId::new(),
            path,
            unity_version,
            created_at: now,
            last_modified: now,
            project_type,
            favorite: false,
            vrc_get: None,
        }
    }

    pub fn path(&self) -> &str {
        self.path.as_ref()
    }

    pub fn name(&self) -> &str {
        Path::new(self.path())
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
    }

    pub fn crated_at(&self) -> DateTime {
        self.created_at
    }

    pub fn last_modified(&self) -> DateTime {
        self.last_modified
    }

    pub fn unity_version(&self) -> Option<UnityVersion> {
        self.unity_version
    }

    pub fn project_type(&self) -> ProjectType {
        self.project_type
    }

    pub fn favorite(&self) -> bool {
        self.favorite
    }

    pub fn set_favorite(&mut self, favorite: bool) {
        self.favorite = favorite;
    }

    pub fn set_unity_version(&mut self, unity_version: UnityVersion) {
        self.unity_version = Some(unity_version);
        if let Some(vrc_get) = self.vrc_get.as_mut() {
            vrc_get.cached_unity_version = Some(unity_version);
            vrc_get.unity_revision = None;
        }
    }

    pub fn set_unity_revision(&mut self, unity_version: UnityVersion, unity_revision: String) {
        self.unity_version = Some(unity_version);
        let vrc_get = self.vrc_get.get_or_insert_with(Default::default);
        vrc_get.cached_unity_version = Some(unity_version);
        vrc_get.unity_revision = Some(unity_revision);
    }

    pub fn unity_revision(&self) -> Option<&str> {
        self.vrc_get
            .as_ref()
            .filter(|x| x.cached_unity_version == self.unity_version)
            .and_then(|x| x.unity_revision.as_deref())
    }

    pub fn custom_unity_args(&self) -> Option<&[String]> {
        self.vrc_get
            .as_ref()
            .and_then(|x| x.custom_unity_args.as_deref())
    }

    pub fn set_custom_unity_args(&mut self, custom_unity_args: Vec<String>) {
        self.vrc_get
            .get_or_insert_with(Default::default)
            .custom_unity_args = Some(custom_unity_args);
    }

    pub fn clear_custom_unity_args(&mut self) {
        self.vrc_get.as_mut().map(|x| x.custom_unity_args = None);
    }
}

use crate::environment::settings::Settings;
use crate::environment::VccDatabaseConnection;
use crate::io::{EnvironmentIo, FileSystemProjectIo, ProjectIo};
use crate::utils::{check_absolute_path, normalize_path};
use crate::version::UnityVersion;
use crate::{io, ProjectType, UnityProject};
use bson::oid::ObjectId;
use bson::DateTime;
use futures::future::join_all;
use log::error;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(crate) static COLLECTION: &str = "projects";

impl VccDatabaseConnection {
    pub async fn migrate(
        &mut self,
        settings: &Settings,
        io: &impl EnvironmentIo,
    ) -> io::Result<()> {
        let projects = settings
            .user_projects()
            .iter()
            .filter(|x| {
                if Path::new(x.as_ref()).is_absolute() {
                    true
                } else {
                    error!("Skipping relative path: {}", x);
                    false
                }
            })
            .map(|x| x.as_ref())
            .collect::<HashSet<_>>();

        let db_projects = self
            .db
            .get_values::<UserProject>(COLLECTION)?
            .into_iter()
            .map(|x| (x.path().to_owned(), x))
            .collect::<HashMap<_, _>>();

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
                let (project_type, unity_version, unity_revision) = get_project_type(
                    io,
                    project.as_ref(),
                )
                .await
                .unwrap_or((ProjectType::Unknown, None, None));
                let mut project = UserProject::new((*project).into(), unity_version, project_type);
                if let (Some(unity), Some(revision)) = (unity_version, unity_revision) {
                    project.set_unity_revision(unity, revision);
                }
                self.db.insert(COLLECTION, &project)?;
            }
        }

        // remove deleted projects
        for (project_path, project) in db_projects.iter() {
            if !projects.contains(project_path.as_str()) {
                self.db.delete(COLLECTION, project.id)?;
            }
        }

        Ok(())
    }
}

impl VccDatabaseConnection {
    pub async fn sync_with_real_projects(
        &mut self,
        skip_not_found: bool,
        io: &impl EnvironmentIo,
    ) -> io::Result<()> {
        let mut projects = self.db.get_values::<UserProject>(COLLECTION)?;

        let changed_projects = join_all(
            projects
                .iter_mut()
                .map(|x| update_project_with_actual_data(io, x, skip_not_found)),
        )
        .await;

        for project in changed_projects.iter().flatten() {
            self.db.update(COLLECTION, project)?;
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
        let projects = self.db.get_values::<UserProject>(COLLECTION)?;

        let mut projects_by_path = HashMap::<_, Vec<_>>::new();

        for project in &projects {
            projects_by_path
                .entry(project.path())
                .or_default()
                .push(project);
        }

        for (_, values) in projects_by_path {
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
                self.db.update(COLLECTION, &project)?;
            }

            // remove rest
            for project in values.iter().skip(1) {
                self.db.delete(COLLECTION, project.id)?;
            }
        }

        Ok(())
    }

    pub fn get_projects(&self) -> io::Result<Vec<UserProject>> {
        Ok(self.db.get_values(COLLECTION)?)
    }

    pub fn find_project(&self, project_path: &Path) -> io::Result<Option<UserProject>> {
        check_absolute_path(project_path)?;
        let project_path = normalize_path(project_path);

        let project = self.db.get_values::<UserProject>(COLLECTION)?;
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
        Ok(self.db.update(COLLECTION, &project)?)
    }

    pub fn remove_project(&mut self, project: &UserProject) -> io::Result<()> {
        self.db.delete(COLLECTION, project.id)?;
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

        self.db.insert(COLLECTION, &new_project)?;

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
    unity_path: Option<String>,
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
        if let Some(x) = self.vrc_get.as_mut() {
            x.custom_unity_args = None;
        }
    }

    pub fn unity_path(&self) -> Option<&str> {
        self.vrc_get.as_ref().and_then(|x| x.unity_path.as_deref())
    }

    pub fn set_unity_path(&mut self, unity_path: String) {
        self.vrc_get.get_or_insert_with(Default::default).unity_path = Some(unity_path);
    }

    pub fn clear_unity_path(&mut self) {
        if let Some(x) = self.vrc_get.as_mut() {
            x.unity_path = None;
        }
    }
}

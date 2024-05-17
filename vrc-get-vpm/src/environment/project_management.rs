use crate::io::{EnvironmentIo, FileSystemProjectIo, ProjectIo};
use crate::utils::PathBufExt;
use crate::version::UnityVersion;
use crate::{io, Environment, HttpClient, ProjectType, UnityProject};
use bson::oid::ObjectId;
use bson::DateTime;
use futures::future::join_all;
use log::error;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
                ) -> io::Result<(ProjectType, Option<UnityVersion>)> {
                    let project = UnityProject::load(io.new_project_io(path)).await?;
                    let detected_type = project.detect_project_type().await?;
                    Ok((detected_type, project.unity_version()))
                }
                let (project_type, unity_version) = get_project_type(&self.io, project.as_ref())
                    .await
                    .unwrap_or((ProjectType::Unknown, None));
                db.insert(
                    COLLECTION,
                    &UserProject::new((*project).into(), unity_version, project_type),
                )?;
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

            let mut changed = false;

            let loaded_project = UnityProject::load(io.new_project_io(path)).await?;
            if let Some(unity_version) = loaded_project.unity_version() {
                if project.unity_version() != Some(unity_version) {
                    changed = true;
                    project.unity_version = Some(unity_version);
                }
            }

            let project_type = loaded_project.detect_project_type().await?;
            if project.project_type() != project_type {
                changed = true;
                project.project_type = project_type;
            }

            Ok(if changed { Some(project) } else { None })
        }

        Ok(())
    }

    // TODO: return wrapper type instead?
    pub fn get_projects(&self) -> io::Result<Vec<UserProject>> {
        Ok(self.get_db()?.get_values(COLLECTION)?)
    }

    pub fn update_project_last_modified(&mut self, project_path: &Path) -> io::Result<()> {
        let db = self.get_db()?;
        let project_path = if project_path.is_absolute() {
            normalize_path(project_path)
        } else {
            normalize_path(&std::env::current_dir().unwrap().joined(project_path))
        };

        let mut project = db.get_values::<UserProject>(COLLECTION)?;
        let Some(project) = project
            .iter_mut()
            .find(|x| Path::new(x.path()) == project_path)
        else {
            return Ok(());
        };

        project.last_modified = DateTime::now();
        db.update(COLLECTION, project)?;

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
        let path = project.project_dir();
        let path = if path.is_absolute() {
            normalize_path(path)
        } else {
            normalize_path(&std::env::current_dir().unwrap().joined(path))
        };
        let path = path.to_str().ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "project path is not utf8",
        ))?;
        let unity_version = project.unity_version().ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "project has no unity version",
        ))?;

        let project_type = project.detect_project_type().await?;

        let new_project = UserProject::new(path.into(), Some(unity_version), project_type);

        self.get_db()?.insert(COLLECTION, &new_project)?;
        self.settings.add_user_project(path);

        Ok(())
    }
}

fn normalize_path(input: &Path) -> PathBuf {
    let mut result = PathBuf::with_capacity(input.as_os_str().len());

    for component in input.components() {
        match component {
            Component::Prefix(prefix) => result.push(prefix.as_os_str()),
            Component::RootDir => result.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(_) => result.push(component.as_os_str()),
        }
    }

    result
}

#[derive(Serialize, Deserialize)]
pub struct UserProject {
    #[serde(rename = "_id")]
    id: ObjectId,
    #[serde(rename = "Path")]
    path: Box<str>,
    #[serde(rename = "UnityVersion")]
    unity_version: Option<UnityVersion>,
    #[serde(rename = "CreatedAt")]
    created_at: DateTime,
    #[serde(rename = "LastModified")]
    last_modified: DateTime,
    #[serde(rename = "Type")]
    project_type: ProjectType,
    #[serde(rename = "Favorite")]
    favorite: bool,
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
}

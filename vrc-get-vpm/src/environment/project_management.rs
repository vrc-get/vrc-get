use crate::io::{EnvironmentIo, FileSystemProjectIo, ProjectIo};
use crate::utils::PathBufExt;
use crate::version::UnityVersion;
use crate::{io, Environment, HttpClient, ProjectType, UnityProject};
use futures::future::join_all;
use log::error;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use vrc_get_litedb::{DateTime, Project};

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
            .get_projects()?
            .into_vec()
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
                db.insert_project(&Project::new(
                    (*project).into(),
                    unity_version.map(|x| x.to_string().into()),
                    project_type.into(),
                ))?;
            }
        }

        // remove deleted projects
        for project in db_projects.iter() {
            if !projects.contains(project.0.as_str()) {
                db.delete_project(project.1.id())?;
            }
        }

        Ok(())
    }

    pub async fn sync_with_real_projects(&mut self, skip_not_found: bool) -> io::Result<()> {
        let db = self.get_db()?; // ensure the database connection is initialized

        let mut projects = db.get_projects()?;

        let changed_projects = join_all(
            projects
                .iter_mut()
                .map(|x| update_project_with_actual_data(&self.io, x, skip_not_found)),
        )
        .await;

        for project in changed_projects.iter().flatten() {
            db.update_project(project)?;
        }

        async fn update_project_with_actual_data<'a>(
            io: &impl EnvironmentIo,
            project: &'a mut Project,
            skip_not_found: bool,
        ) -> Option<&'a Project> {
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
            project: &'a mut Project,
            skip_not_found: bool,
        ) -> io::Result<Option<&'a Project>> {
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
                let unity_version = unity_version.to_string().into_boxed_str();
                if project.unity_version() != Some(&unity_version) {
                    changed = true;
                    project.set_unity_version(Some(unity_version));
                }
            }

            let project_type = loaded_project.detect_project_type().await?;
            if project.project_type() != project_type.into() {
                changed = true;
                project.set_project_type(project_type.into());
            }

            Ok(if changed { Some(project) } else { None })
        }

        Ok(())
    }

    // TODO: return wrapper type instead?
    pub fn get_projects(&self) -> io::Result<Vec<UserProject>> {
        Ok(self
            .get_db()?
            .get_projects()?
            .into_vec()
            .into_iter()
            .map(UserProject::new)
            .collect())
    }

    pub fn update_project_last_modified(&mut self, project_path: &Path) -> io::Result<()> {
        let db = self.get_db()?;
        let project_path = if project_path.is_absolute() {
            normalize_path(project_path)
        } else {
            normalize_path(&std::env::current_dir().unwrap().joined(project_path))
        };

        let mut project = db.get_projects()?;
        let Some(project) = project
            .iter_mut()
            .find(|x| Path::new(x.path()) == project_path)
        else {
            return Ok(());
        };

        project.set_last_modified(DateTime::now());
        db.update_project(project)?;

        Ok(())
    }

    pub fn update_project(&mut self, project: &UserProject) -> io::Result<()> {
        Ok(self.get_db()?.update_project(&project.project)?)
    }

    pub fn remove_project(&mut self, project: &UserProject) -> io::Result<()> {
        let db = self.get_db()?;

        db.delete_project(project.project.id())?;
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

        let new_project = Project::new(
            path.into(),
            unity_version.to_string().into_boxed_str().into(),
            project_type.into(),
        );

        self.get_db()?.insert_project(&new_project)?;
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

pub struct UserProject {
    project: Project,
}

impl UserProject {
    fn new(project: Project) -> Self {
        Self { project }
    }

    pub fn path(&self) -> &str {
        self.project.path()
    }

    pub fn name(&self) -> &str {
        Path::new(self.path())
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
    }

    pub fn crated_at(&self) -> DateTime {
        self.project.created_at()
    }

    pub fn last_modified(&self) -> DateTime {
        // TODO: provide our wrapper type
        self.project.last_modified()
    }

    pub fn unity_version(&self) -> Option<UnityVersion> {
        UnityVersion::parse(self.project.unity_version()?)
    }

    pub fn project_type(&self) -> ProjectType {
        self.project.project_type().into()
    }

    pub fn favorite(&self) -> bool {
        self.project.favorite()
    }

    pub fn set_favorite(&mut self, favorite: bool) {
        self.project.set_favorite(favorite);
    }
}

use crate::io::{EnvironmentIo, FileSystemProjectIo, ProjectIo};
use crate::utils::PathBufExt;
use crate::version::UnityVersion;
use crate::{io, Environment, HttpClient, ProjectType, UnityProject};
use futures::future::try_join_all;
use log::error;
use std::path::{Component, Path, PathBuf};
use vrc_get_litedb::{DatabaseConnection, DateTime, Project};

impl<T: HttpClient, IO: EnvironmentIo> Environment<T, IO> {
    // TODO?: use inner mutability to get the database connection?
    fn get_db(&mut self) -> io::Result<&DatabaseConnection> {
        if self.litedb_connection.is_none() {
            self.litedb_connection = Some(self.io.connect_lite_db()?);
        }

        Ok(self.litedb_connection.as_ref().unwrap())
    }

    pub async fn sync_with_real_projects(&mut self) -> io::Result<()> {
        self.get_db()?; // ensure the database connection is initialized
        let db = self.litedb_connection.as_ref().unwrap();

        let mut projects = db.get_projects()?;

        let changed_projects = try_join_all(
            projects
                .iter_mut()
                .map(|x| update_project_with_actual_data(&self.io, x)),
        )
        .await?;

        for project in changed_projects.iter().flatten() {
            db.update_project(project)?;
        }

        async fn update_project_with_actual_data<'a>(
            io: &impl EnvironmentIo,
            project: &'a mut Project,
        ) -> io::Result<Option<&'a Project>> {
            let path = project.path().as_ref();

            let metadata = io.metadata(path).await;
            if !metadata.map(|x| x.is_dir()).unwrap_or(false) {
                error!("Project {} not found", path.display());
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
    pub fn get_projects(&mut self) -> io::Result<Vec<UserProject>> {
        Ok(self
            .get_db()?
            .get_projects()?
            .into_vec()
            .into_iter()
            .map(UserProject::new)
            .collect())
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
            Component::RootDir => result.push("/"),
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
}

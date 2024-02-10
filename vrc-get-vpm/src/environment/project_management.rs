use crate::io::{EnvironmentIo, FileSystemProjectIo, ProjectIo};
use crate::version::UnityVersion;
use crate::{io, Environment, HttpClient, ProjectType, UnityProject};
use std::path::Path;
use vrc_get_litedb::{DatabaseConnection, DateTime, Project};

impl<T: HttpClient, IO: EnvironmentIo> Environment<T, IO> {
    // TODO?: use inner mutability to get the database connection?
    fn get_db(&mut self) -> io::Result<&DatabaseConnection> {
        if self.litedb_connection.is_none() {
            self.litedb_connection = Some(self.io.connect_lite_db()?);
        }

        Ok(self.litedb_connection.as_ref().unwrap())
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

    pub fn add_project<ProjectIO: ProjectIo + FileSystemProjectIo>(
        &mut self,
        project: &UnityProject<ProjectIO>,
    ) -> io::Result<()> {
        let path = project.project_dir().to_str().ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "project path is not utf8",
        ))?;
        let unity_version = project.unity_version().ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "project has no unity version",
        ))?;

        let project_type = project.detect_project_type()?;

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

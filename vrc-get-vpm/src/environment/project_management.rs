use crate::io::{EnvironmentIo, FileSystemProjectIo, ProjectIo};
use crate::{io, Environment, HttpClient, UnityProject};
use vrc_get_litedb::{DatabaseConnection, Project, ProjectType};

impl<T: HttpClient, IO: EnvironmentIo> Environment<T, IO> {
    // TODO?: use inner mutability to get the database connection?
    fn get_db(&mut self) -> io::Result<&DatabaseConnection> {
        if self.litedb_connection.is_none() {
            self.litedb_connection = Some(self.io.connect_lite_db()?);
        }

        Ok(self.litedb_connection.as_ref().unwrap())
    }

    // TODO: return wrapper type instead?
    pub fn get_projects(&mut self) -> io::Result<Box<[Project]>> {
        Ok(self.get_db()?.get_projects()?)
    }

    pub fn remove_project(&mut self, path: &str) -> io::Result<usize> {
        let db = self.get_db()?;
        let mut count = 0;

        for x in db.get_projects()?.iter().filter(|x| x.path() == path) {
            db.delete_project(x.id())?;
            count += 1;
        }

        // remove from settings json
        self.settings.remove_user_project(path);

        Ok(count)
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

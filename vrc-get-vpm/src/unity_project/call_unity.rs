use crate::io;
use crate::io::{FileSystemProjectIo, ProjectIo};
use crate::UnityProject;
use std::path::Path;

#[non_exhaustive]
#[derive(Debug)]
pub enum ExecuteUnityError {
    Io(io::Error),
    Unity(io::ExitStatus),
}

impl std::error::Error for ExecuteUnityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteUnityError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl std::fmt::Display for ExecuteUnityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecuteUnityError::Io(err) => write!(f, "{}", err),
            ExecuteUnityError::Unity(status) => {
                write!(f, "Unity exited with status {}", status)
            }
        }
    }
}

impl From<io::Error> for ExecuteUnityError {
    fn from(err: io::Error) -> Self {
        ExecuteUnityError::Io(err)
    }
}

type Result<T = (), E = ExecuteUnityError> = std::result::Result<T, E>;

impl<IO> UnityProject<IO>
where
    IO: ProjectIo + FileSystemProjectIo,
{
    pub async fn call_unity(&self, unity_executable: &Path) -> Result {
        let status = self
            .io
            .command_status(
                unity_executable.as_ref(),
                &[
                    "-quit".as_ref(),
                    "-batchmode".as_ref(),
                    "-projectPath".as_ref(),
                    self.project_dir().as_os_str(),
                ],
            )
            .await?;

        if !status.success() {
            return Err(ExecuteUnityError::Unity(status));
        }

        Ok(())
    }

    pub async fn launch_gui_unity_detached(&self, unity_executable: &Path) -> io::Result<()> {
        self.io
            .spawn_detached(
                unity_executable.as_ref(),
                &["-projectPath".as_ref(), self.project_dir().as_os_str()],
            )
            .await?;

        Ok(())
    }
}

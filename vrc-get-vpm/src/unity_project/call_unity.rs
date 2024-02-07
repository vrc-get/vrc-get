use crate::io::DefaultProjectIo;
use crate::UnityProject;
use std::path::Path;
use tokio::io;
use tokio::process::Command;

#[non_exhaustive]
#[derive(Debug)]
pub enum ExecuteUnityError {
    Io(io::Error),
    Unity(std::process::ExitStatus),
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

impl UnityProject<DefaultProjectIo> {
    pub async fn call_unity(&self, unity_executable: &Path) -> Result {
        let mut command = Command::new(unity_executable);
        command.args(["-quit", "-batchmode", "-projectPath"]);
        command.arg(self.project_dir());
        let status = command.status().await?;

        if !status.success() {
            return Err(ExecuteUnityError::Unity(status));
        }

        Ok(())
    }
}

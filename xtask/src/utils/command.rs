use anyhow::{bail, Context, Result};
use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};

pub trait CommandExt {
    fn run_checked(&mut self, what: &str) -> Result<()>;
    fn run_capture_checked(&mut self, what: &str) -> Result<String>;
    fn display_command(&self) -> String;
}

impl CommandExt for Command {
    fn run_checked(&mut self, what: &str) -> Result<()> {
        self.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        let printable = self.display_command();
        let status = self
            .status()
            .with_context(|| format!("{what}: failed to start ({printable})"))?;
        if !status.success() {
            bail!("{what}: command failed with {status} ({printable})");
        }
        Ok(())
    }

    fn run_capture_checked(&mut self, what: &str) -> Result<String> {
        let printable = self.display_command();
        let output = self
            .output()
            .with_context(|| format!("{what}: failed to start ({printable})"))?;

        if !output.status.success() {
            bail!(
                "{what}: command failed with {} ({printable})",
                output.status
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn display_command(&self) -> String {
        let mut parts = vec![self.get_program().to_string_lossy().into_owned()];
        parts.extend(
            self.get_args()
                .map(OsStr::to_string_lossy)
                .map(|x| x.into_owned()),
        );
        parts.join(" ")
    }
}

/// Runs command under wine environment if exists
pub struct WineRunner {
    wine: Option<String>,
}

impl WineRunner {
    pub fn detect() -> Result<Self> {
        // if specified manually via environment variable, use it
        if let Some(value) = std::env::var_os("WINE") {
            let value = value.to_string_lossy().into_owned();
            if value.is_empty() {
                return Ok(Self { wine: None });
            }
            return Ok(Self { wine: Some(value) });
        }

        if command_exists("wine64")? {
            return Ok(Self {
                wine: Some("wine64".to_owned()),
            });
        }

        if command_exists("wine")? {
            return Ok(Self {
                wine: Some("wine".to_owned()),
            });
        }

        Ok(Self { wine: None })
    }

    /// Runs specified command inside wine environment
    pub fn command(&self, program: &Path) -> Command {
        if let Some(wine) = &self.wine {
            let mut cmd = Command::new(wine);
            // pass command as-is. solve it in wine
            cmd.arg(program);
            cmd
        } else {
            Command::new(program)
        }
    }

    /// Converts specified path to windows-compatible path.
    ///
    /// This function only converts absolute path.
    pub fn path(&self, path: &Path) -> String {
        if self.wine.is_none() {
            return path.as_os_str().to_string_lossy().into_owned();
        }

        if !path.is_absolute() {
            return path.as_os_str().to_string_lossy().into_owned();
        }

        let path = path.as_os_str().to_string_lossy().replace('/', "\\");
        format!("Z:{path}")
    }
}

pub fn command_exists(program: &str) -> Result<bool> {
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Ok(true);
        }
        if cfg!(windows) {
            let candidate_exe = dir.join(format!("{program}.exe"));
            if candidate_exe.is_file() {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

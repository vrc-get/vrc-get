use anyhow::{Context, Result, bail};
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

pub fn create_command(cmd_name: impl AsRef<OsStr>) -> Command {
    pub fn create_command(cmd_name: &OsStr) -> Command {
        #[cfg(windows)]
        {
            // Windowsの場合、PATHEXTを考慮して実行ファイルの実体を探す
            if let Some(resolved_path) = find_executable_windows(cmd_name) {
                // 見つかった場合はそのフルパス（または拡張子付き）を使用
                return Command::new(resolved_path);
            }
        }

        Command::new(cmd_name)
    }

    create_command(cmd_name.as_ref())
}

#[cfg(windows)]
fn find_executable_windows(cmd_name: &OsStr) -> Option<std::path::PathBuf> {
    use std::env;
    use std::ffi::OsString;
    use std::path::PathBuf;

    let cmd_path = Path::new(cmd_name);

    // If there already is extension in their file name, we won't insert new extension
    if cmd_path.extension().is_some() {
        return Some(cmd_path.to_path_buf()).take_if(|_| cmd_path.exists());
    }

    let pathext = env::var_os("PATHEXT").unwrap_or_else(|| OsString::from(".EXE;.BAT;.CMD"));
    let extensions: Vec<_> = env::split_paths(&pathext).collect();

    // When the path contains path separator, we only try appending extension.
    if cmd_name.as_encoded_bytes().contains(&b'\\') || cmd_name.as_encoded_bytes().contains(&b'/') {
        return find_with_extensions(cmd_path, &extensions);
    }

    // If the cmd_name consists only with filename, we find CWD and PATH environment variable.

    if let Some(found) = find_with_extensions(&Path::new(".").join(cmd_path), &extensions) {
        return Some(found);
    }

    if let Some(paths) = env::var_os("PATH") {
        for path in env::split_paths(&paths) {
            if let Some(found) = find_with_extensions(&path.join(cmd_path), &extensions) {
                return Some(found);
            }
        }
    }
    return None;

    #[cfg(windows)]
    fn find_with_extensions(base_path: &Path, extensions: &[PathBuf]) -> Option<PathBuf> {
        for ext in extensions {
            let ext_str = ext.to_string_lossy();
            let mut candidate = base_path.to_path_buf();
            candidate.set_extension(ext_str.trim_start_matches('.'));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }
}

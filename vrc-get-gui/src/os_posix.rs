//! OS-specific functionality.

use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use nix::fcntl::{Flock, FlockArg};

pub(crate) use os_more::{CAN_DETECT_UNITY_EDITOR_READY, start_command};

pub(crate) fn can_bring_unity_to_front() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some()
    }
    #[cfg(not(target_os = "linux"))]
    {
        os_more::CAN_BRING_UNITY_TO_FRONT
    }
}

pub(crate) struct UnityRuntimeCache;

impl UnityRuntimeCache {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn is_editor_ready(&mut self, _project_path: &Path) -> bool {
        false
    }

    pub(crate) fn bring_unity_to_front(
        &mut self,
        project_path: &Path,
    ) -> io::Result<super::BringUnityToFrontResult> {
        os_more::bring_unity_to_front(project_path)
    }

    pub(crate) fn invalidate(&mut self) {}
}

async fn start_command_posix(_: &OsStr, path: &OsStr, args: &[&OsStr]) -> std::io::Result<()> {
    let mut command = Command::new(path);
    command.args(args);
    os_more::fix_env_variables(&mut command);
    command.process_group(0);
    let mut process = command.spawn()?;
    std::thread::spawn(move || process.wait());
    Ok(())
}

pub(crate) fn is_locked(path: &Path) -> io::Result<bool> {
    let file = OpenOptions::new().read(true).open(path)?;

    match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
        Ok(_flock) => Ok(false), // acquired lock → not locked by Unity
        Err((_file, errno)) if errno == nix::errno::Errno::EWOULDBLOCK => Ok(true),
        Err((_file, errno)) => Err(errno.into()),
    }
}

#[cfg(target_os = "macos")]
#[path = "os_macos.rs"]
mod os_more;

#[cfg(target_os = "linux")]
#[path = "os_linux.rs"]
mod os_more;

pub fn os_info() -> &'static str {
    static OS_INFO: OnceLock<String> = OnceLock::new();
    OS_INFO.get_or_init(os_more::compute_os_info)
}

pub use os_more::initialize;
pub use os_more::is_noexec;
pub use os_more::open_that;

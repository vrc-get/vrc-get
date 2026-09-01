//! OS-specific functionality.

use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io;
use std::os::unix::prelude::*;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

#[cfg(not(target_os = "linux"))]
use nix::libc::{F_UNLCK, c_short, flock};

pub(crate) use os_more::{CAN_BRING_UNITY_TO_FRONT, CAN_DETECT_UNITY_EDITOR_READY, start_command};

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

    // Linux: Unity uses flock (BSD locks), which are independent from fcntl (POSIX locks).
    #[cfg(target_os = "linux")]
    {
        let fd = file.as_raw_fd();
        let ret = unsafe { nix::libc::flock(fd, nix::libc::LOCK_EX | nix::libc::LOCK_NB) };
        if ret == -1 {
            let err = io::Error::last_os_error();
            if err.raw_os_error() == Some(nix::libc::EWOULDBLOCK) {
                return Ok(true);
            }
            return Err(err);
        }
        unsafe { nix::libc::flock(fd, nix::libc::LOCK_UN) };
        Ok(false)
    }

    // macOS: Darwin unifies fcntl and flock, so fcntl F_GETLK works.
    #[cfg(not(target_os = "linux"))]
    {
        let mut lock = flock {
            l_start: 0,
            l_len: 0,
            l_pid: 0,
            l_type: F_UNLCK as c_short, // macOS denies l_type: 0
            l_whence: 0,
        };
        nix::fcntl::fcntl(&file, nix::fcntl::F_GETLK(&mut lock))?;
        Ok(lock.l_type != F_UNLCK as c_short)
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

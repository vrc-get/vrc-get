//! OS-specific functionality.

use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io;
use std::os::unix::prelude::*;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use nix::libc::{F_UNLCK, c_short, flock};

pub(crate) use os_more::start_command;

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
    let mut lock = flock {
        l_start: 0,
        l_len: 0,
        l_pid: 0,
        l_type: F_UNLCK as c_short, // macOS denies l_type: 0
        l_whence: 0,
    };
    let file = OpenOptions::new().read(true).open(path)?;

    nix::fcntl::fcntl(file, nix::fcntl::F_GETLK(&mut lock))?;

    Ok(lock.l_type != F_UNLCK as c_short)
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
pub use os_more::open_that;

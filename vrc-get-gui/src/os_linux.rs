// linux-specific functionality.

use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::prelude::OsStrExt;
use std::process::{Command, Stdio};
use std::sync::Arc;

use arc_swap::ArcSwapOption;
use nix::libc::uname;
use tauri::Manager;

pub(crate) async fn start_command(name: &OsStr, path: &OsStr, args: &[&OsStr]) -> io::Result<()> {
    super::start_command_posix(name, path, args).await
}

pub(super) fn compute_os_info() -> String {
    let kernel = kernel_version();

    if let Some(description) = lsb_release_description() {
        format!("Linux {kernel}, {description}")
    } else if let Some(pretty_name) = os_release() {
        format!("Linux {kernel}, {pretty_name}")
    } else {
        format!("Linux {kernel}, Unknown Distribution")
    }
}

fn kernel_version() -> String {
    unsafe {
        let mut utsname = std::mem::zeroed();
        if uname(&mut utsname) == -1 {
            return "(unknown kernel)".into();
        }
        let version = std::ffi::CStr::from_ptr(utsname.release.as_ptr());
        version.to_string_lossy().into_owned()
    }
}

fn lsb_release_description() -> Option<String> {
    // lsb_release is defined in wide versions of Linux Base Standard
    // https://refspecs.linuxfoundation.org/LSB_1.0.0/gLSB/lsbrelease.html (with no specific format)
    // https://refspecs.linuxfoundation.org/LSB_2.0.0/LSB-Core/LSB-Core/lsbrelease.html (first version with format)
    // https://refspecs.linuxfoundation.org/LSB_5.0.0/LSB-Core-generic/LSB-Core-generic/lsbrelease.html (latest LSB)
    let output = Command::new("lsb_release").arg("-d").output().ok()?;
    let description = String::from_utf8(output.stdout).ok()?;
    static PREFIX: &str = "Description:\t";
    let line = description.lines().find(|x| x.starts_with(PREFIX))?;
    let line = line.trim_start_matches(PREFIX);
    Some(line.trim().into())
}

fn os_release() -> Option<String> {
    // /etc/os-release is a standard file in modern Linux distributions
    // https://www.freedesktop.org/software/systemd/man/latest/os-release.html
    let file = std::fs::read_to_string("/etc/os-release").ok()?;

    static PRETTY_NAME: &str = "PRETTY_NAME=";

    if let Some(pretty_name) = (file.lines().rev())
        .find(|x| x.starts_with(PRETTY_NAME))
        .map(|x| x.trim_start_matches(PRETTY_NAME).trim_matches('"'))
    {
        return Some(pretty_name.into());
    }

    static NAME: &str = "NAME=";
    static VERSION: &str = "VERSION=";

    let name = (file.lines().rev())
        .find(|x| x.starts_with(NAME))
        .map(|x| x.trim_start_matches(NAME).trim_matches('"'));
    let version = (file.lines().rev())
        .find(|x| x.starts_with(VERSION))
        .map(|x| x.trim_start_matches(VERSION).trim_matches('"'));

    match (name, version) {
        (Some(name), Some(version)) => Some(format!("{} {}", name, version)),
        (Some(name), None) => Some(name.into()),
        _ => None,
    }
}

static APPDIR: ArcSwapOption<OsString> = ArcSwapOption::const_empty();

pub fn initialize(app_handle: tauri::AppHandle) {
    APPDIR.store(app_handle.env().appdir.clone().map(Arc::from));
}

pub fn open_that(path: impl AsRef<OsStr>) -> io::Result<()> {
    // We implement open_that here since we have to fix env variables for xdg-open.
    // This implementation supports xdg-open only but it's general enough for most cases, I think

    let mut command = Command::new("xdg-open");
    command.arg(path);
    fix_env_variables(&mut command);
    let status = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if !status.success() {
        return Err(io::Error::other(format!(
            "Launcher xdg-open failed with {:?}",
            status
        )));
    }

    Ok(())
}

pub(super) fn fix_env_variables(command: &mut Command) {
    // NOTE: this does not handle env_clear correctly

    let appdir = APPDIR.load();
    let Some(appdir) = appdir.as_ref() else {
        return;
    };
    let appdir = appdir.as_ref();
    let appdir_bytes = appdir.as_bytes();

    if appdir_bytes.contains(&b':') || appdir_bytes.is_empty() {
        // if the appdir contains ':',
        // the PATH variables become wired (broken) and impossible to fix so keep as-is
        return;
    }

    // remove appimage specific variables
    command.env_remove("ARGV0");
    command.env_remove("APPIMAGE");
    command.env_remove("APPDIR");

    // PYTHONHOME is set by AppRun for compability
    command.env_remove("PYTHONHOME");

    let manually_sets = command
        .get_envs()
        .map(|(n, _)| n.to_os_string())
        .collect::<HashSet<_>>();

    // process path-like variables (remove appdir-related paths)
    // see https://github.com/AppImage/AppImageKit/blob/e8dadbb09fed3ae3c3d5a5a9ba2c47a072f71c40/src/AppRun.c#L171-L194
    // LD_LIBRARY_PATH is necessary for xdg-open to work correctly
    for (var_name, current) in std::env::vars_os() {
        if manually_sets.contains(var_name.as_os_str()) {
            continue; // do not change manually specified variables
        }

        // in this case, the variable will be inherited from the environment

        let current_bytes = current.as_bytes();

        if current_bytes.starts_with(appdir_bytes) {
            // in this case, the variable was modified to start with appdir
            // therefore, we need to remove the appdir-related path

            let mut current_bytes = current_bytes;
            while current_bytes.starts_with(appdir_bytes) {
                if let Some(colon_pos) = current_bytes.iter().position(|&x| x == b':') {
                    current_bytes = &current_bytes[colon_pos + 1..];
                } else {
                    current_bytes = b"";
                }
            }

            if current_bytes != b"" {
                // We successfully removed paths relates to the AppDir so set that value
                command.env(var_name, OsStr::from_bytes(current_bytes));
            } else {
                // In this case, the env variable was not defined and defined only by AppRun so
                // remove the variable
                command.env_remove(var_name);
            }
        }
    }
}

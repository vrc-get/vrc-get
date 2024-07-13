// linux-specific functionality.

use nix::libc::uname;
use std::process::Command;

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

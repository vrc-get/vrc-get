//! OS-specific functionality.

//! This module is for creating `cmd.exe /d /c start "Name"
//! "path/to/executable" args` command correctly.
//!
//! Since the `cmd.exe` has a unique escape sequence behavior,
//! It's necessary to escape the path and arguments correctly.
//!
//! I wrote this module based on [BatBadBut] article.
//!
//! [BatBadBut]: https://flatt.tech/research/posts/batbadbut-you-cant-securely-execute-commands-on-windows/#as-a-developer

use std::ffi::{OsStr, OsString};
use std::fs::OpenOptions;
use std::mem::MaybeUninit;
use std::os::windows::ffi::EncodeWide;
use std::os::windows::prelude::*;
use std::path::Path;
use std::sync::OnceLock;
use std::{io, result};
use tokio::process::Command;
use windows::Win32::Foundation::{ERROR_LOCK_VIOLATION, HANDLE};
use windows::Win32::Storage::FileSystem::{
    LockFileEx, UnlockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LOCK_FILE_FLAGS,
};
use windows::Win32::System::IO::OVERLAPPED;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub(crate) async fn start_command(
    name: &OsStr,
    path: &OsStr,
    args: &[&OsStr],
) -> std::io::Result<()> {
    // prepare
    let mut cmd_args = Vec::new();
    cmd_args.extend("/E:ON /V:OFF /d /c start /b ".encode_utf16());
    append_cmd_escaped(&mut cmd_args, name.encode_wide());
    cmd_args.push(b' ' as u16);

    append_cmd_escaped(&mut cmd_args, path.encode_wide());

    for arg in args {
        cmd_args.push(b' ' as u16);
        append_cmd_escaped(&mut cmd_args, arg.encode_wide());
    }

    // execute
    let status = Command::new("cmd.exe")
        .raw_arg(OsString::from_wide(&cmd_args))
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .await?;

    if !status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "cmd.exe /E:ON /V:OFF /d /c start /d failed with status: {}",
                status
            ),
        ));
    } else {
        Ok(())
    }
}

// %%cd:~,%
const PERCENT_ESCAPED: &[u16] = &[0x25, 0x25, 0x63, 0x64, 0x3a, 0x7e, 0x2c, 0x25];

// based on https://flatt.tech/research/posts/batbadbut-you-cant-securely-execute-commands-on-windows/#as-a-developer
fn append_cmd_escaped(args: &mut Vec<u16>, arg: impl Iterator<Item = u16>) {
    // Enclose the argument with double quotes (").
    args.push('"' as u16);

    let mut backslash = 0;
    for x in arg {
        if x == b'%' as u16 {
            args.extend_from_slice(PERCENT_ESCAPED);
        } else if x == b'"' as u16 {
            // Replace the backslash (\) in front of the double quote (") with two backslashes (\\).
            //  To implement that, append the backslashes again
            args.extend(std::iter::repeat(b'\\' as u16).take(backslash));
            // Replace the double quote (") with two double quotes ("").
            args.push(b'"' as u16);
            args.push(b'"' as u16);
        } else if x == '\n' as u16 {
            // Remove newline characters (\n).
        } else {
            args.push(x);
        }

        // count b'\\'
        if x == b'\\' as u16 {
            backslash += 1;
        } else {
            backslash = 0;
        }
    }

    // Enclose the argument with double quotes (").
    args.push('"' as u16);
}

pub(crate) fn is_locked(path: &Path) -> io::Result<bool> {
    let file = OpenOptions::new().read(true).open(path)?;
    unsafe {
        let mut overlapped: OVERLAPPED = MaybeUninit::zeroed().assume_init();
        overlapped.Anonymous.Anonymous.Offset = 0;
        overlapped.Anonymous.Anonymous.OffsetHigh = 0;
        match LockFileEx(
            HANDLE(file.as_raw_handle()),
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            0,
            0,
            &mut overlapped,
        ) {
            Err(ref e) if e.code() == ERROR_LOCK_VIOLATION.into() => {
                // ERROR_LOCK_VIOLATION means it's already locked
                return Ok(false);
            }
            // other error
            Err(e) => return Err(e.into()),
            Ok(()) => {}
        }
        // lock successful; it's not locked so unlock and return true
        let mut overlapped: OVERLAPPED = MaybeUninit::zeroed().assume_init();
        overlapped.Anonymous.Anonymous.Offset = 0;
        overlapped.Anonymous.Anonymous.OffsetHigh = 0;
        UnlockFileEx(HANDLE(file.as_raw_handle()), 0, !0, !0, &mut overlapped)?;
        return Ok(true);
    }
}

pub fn os_info() -> &'static str {
    static OS_INFO: OnceLock<String> = OnceLock::new();

    fn compute_os_info() -> String {
        use windows::Wdk::System::SystemServices::RtlGetVersion;
        use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
        let mut info: OSVERSIONINFOW = Default::default();
        info.dwOSVersionInfoSize = std::mem::size_of_val(&info) as u32;
        unsafe {
            if RtlGetVersion(&mut info).is_err() {
                return "Unknown".to_string();
            }
        }

        let ex_version = &info.szCSDVersion[..];
        let ex_version = &ex_version[..ex_version
            .iter()
            .position(|&x| x == 0)
            .unwrap_or(ex_version.len())];
        let ex_version = String::from_utf16_lossy(ex_version);
        let ex_version = if ex_version.is_empty() {
            "".to_string()
        } else {
            format!(" ({})", ex_version)
        };

        format!(
            "Windows {}.{}.{}{}",
            info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber, ex_version,
        )
    }

    OS_INFO.get_or_init(compute_os_info)
}

pub fn local_app_data() -> &'static str {
    static LOCAL_APP_DATA: OnceLock<String> = OnceLock::new();

    LOCAL_APP_DATA.get_or_init(|| {
        dirs_next::cache_dir()
            .map(|x| x.to_string_lossy().into_owned())
            .unwrap_or_else(|| String::new())
    })
}

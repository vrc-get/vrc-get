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

    let mut buffer = Vec::new();
    for arg in args {
        cmd_args.push(b' ' as u16);
        let arg = arg.encode_wide().collect::<Vec<_>>();
        buffer.clear();
        append_cpp_escaped(&mut buffer, &arg);
        append_cmd_escaped(&mut cmd_args, buffer.iter().copied());
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

/*
/d /c /E:ON /V:OFF start /b "Unity" "C:\Program Files\Unity\Hub\Editor\2022.3.22f1\Editor\Unity.exe" "-projectPath" """D:\VRC\新しいフォルダー (3)\world 2""" "-debugCodeOptimization"
 */

fn append_cpp_escaped(args: &mut Vec<u16>, arg: &[u16]) {
    let need_quote = arg.iter().any(|&c| c == b' ' as u16 || c == b'\t' as u16);
    if need_quote {
        args.push(b'"' as u16);
    }

    let mut backslashes = 0;
    for &x in arg {
        if x == b'\\' as u16 {
            backslashes += 1;
        } else {
            if x == b'"' as u16 {
                // n + 1 backslashes makes n * 2 + 1 backslashes
                args.extend(std::iter::repeat(b'\\' as u16).take(backslashes + 1));
            }
            backslashes = 0;
        }
        args.push(x);
    }

    if need_quote {
        // n backslashes makes n * 2 backslashes
        args.extend(std::iter::repeat(b'\\' as u16).take(backslashes));
        args.push(b'"' as u16);
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
        UnlockFileEx(
            HANDLE(file.as_raw_handle()),
            0,
            !0,
            !0,
            &mut overlapped,
        )?;
        return Ok(true);
    }
}

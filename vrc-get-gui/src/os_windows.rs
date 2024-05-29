//! OS-specific functionality.

//! This module is for creating `cmd.exe /d /c start "Name"
//! "path/to/executable" args` command correctly.
//!
//! Since the `cmd.exe` has a unique escape sequence behavior,
//! It's necessary to escape the path and arguments correctly.
//!
//! I wrote this module based on [research by Y.m Ryota][research-zenn].
//!
//! [research-zenn]: https://zenn.dev/tryjsky/articles/0610b2f32453e7

use std::ffi::{OsStr, OsString};
use std::fs::OpenOptions;
use std::mem::MaybeUninit;
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
    let percent_env_name = "PERCENT".encode_utf16().collect::<Vec<_>>();

    let mut cmd_args = Vec::new();
    cmd_args.extend("/d /c start /b \"".encode_utf16());
    cmd_args.extend(name.encode_wide());
    cmd_args.push(b'"' as u16);
    cmd_args.push(b' ' as u16);

    // since pathname cannot have '"' in it, we don't need to escape it
    cmd_args.push('"' as u16);
    append_cmd_no_caret_escape(
        &mut cmd_args,
        path.encode_wide().collect::<Vec<_>>().as_slice(),
        &percent_env_name,
    );
    cmd_args.push('"' as u16);

    let mut buffer = Vec::new();
    for arg in args {
        cmd_args.push(b' ' as u16);
        let arg = arg.encode_wide().collect::<Vec<_>>();
        buffer.clear();
        append_cpp_escaped(&mut buffer, &arg);
        append_cmd_escaped(&mut cmd_args, &buffer, &percent_env_name);
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
            format!("cmd.exe /d /c start /d failed with status: {}", status),
        ));
    } else {
        Ok(())
    }
}

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

// ' ' (whitespace), '=', ';', ',', '<', '>', '|', '&', '^', '(', ')', '!', '"', '@'
// We need another escape for '%'
const ESCAPE_CHARS: &[u16] = &[
    0x20, 0x3d, 0x3b, 0x2c, 0x3c, 0x3e, 0x7c, 0x26, 0x5e, 0x28, 0x29, 0x21, 0x22, 0x40,
];

fn append_cmd_escaped(args: &mut Vec<u16>, arg: &[u16], percent_env_var_name: &[u16]) {
    if arg.first().copied() == Some('"' as u16) && arg.last().copied() == Some('"' as u16) {
        // it's "-quoted, so we don't need to escape if there is no '"' inside
        let contains_quote = arg.iter().filter(|&&x| x == '"' as u16).count() > 2;
        if contains_quote {
            append_cmd_caret_escaped(args, arg, percent_env_var_name);
        } else {
            append_cmd_no_caret_escape(args, arg, percent_env_var_name);
        }
    } else if arg.iter().any(|x| ESCAPE_CHARS.contains(x)) {
        if !arg.iter().any(|&x| x == '"' as u16) {
            // if contains escape chars but not ", we can use "-quoting
            args.push(b'"' as u16);
            append_cmd_no_caret_escape(args, arg, percent_env_var_name);
            args.push(b'"' as u16);
        } else {
            // if contains ", we have to use caret-escaping
            append_cmd_caret_escaped(args, arg, percent_env_var_name);
        }
    } else {
        // no escape is needed
        append_cmd_no_caret_escape(args, arg, percent_env_var_name);
    }
}

fn append_cmd_no_caret_escape(args: &mut Vec<u16>, arg: &[u16], percent_env_var_name: &[u16]) {
    // even without ^-escaping, we need to escape '%' since env var expansion is proceeded even
    // inside '"'-quoted string
    for &x in arg {
        if x == b'%' as u16 {
            args.push(b'%' as u16);
            args.extend_from_slice(percent_env_var_name);
            args.push(b'%' as u16);
        } else {
            args.push(x);
        }
    }
}

fn append_cmd_caret_escaped(args: &mut Vec<u16>, arg: &[u16], percent_env_var_name: &[u16]) {
    for &x in arg {
        if x == b'%' as u16 {
            args.push(b'%' as u16);
            args.extend_from_slice(percent_env_var_name);
            args.push(b'%' as u16);
        } else if ESCAPE_CHARS.contains(&x) {
            args.push(b'^' as u16);
            args.push(x);
        } else {
            args.push(x);
        }
    }
}

pub(crate) fn is_locked(path: &Path) -> io::Result<bool> {
    let file = OpenOptions::new().read(true).open(path)?;
    unsafe {
        let mut overlapped: OVERLAPPED = MaybeUninit::zeroed().assume_init();
        overlapped.Anonymous.Anonymous.Offset = 0;
        overlapped.Anonymous.Anonymous.OffsetHigh = 0;
        match LockFileEx(
            HANDLE(file.as_raw_handle() as isize),
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
            HANDLE(file.as_raw_handle() as isize),
            0,
            !0,
            !0,
            &mut overlapped,
        )?;
        return Ok(true);
    }
}

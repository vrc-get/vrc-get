mod find_unity_from_unity_hub_logic;
mod os;

use crate::io;
use crate::version::UnityVersion;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::str::from_utf8;
use tokio::process::Command;

pub use find_unity_from_unity_hub_logic::load_unity_by_loading_unity_hub_files;
pub use os::load_unity_version;

/// Returns the path to executable file
///
/// On macOS, this function expects Path to .app file and returns binary file.
/// On other platforms, does nothing
pub fn get_executable_path(path: &Path) -> Cow<'_, Path> {
    #[cfg(not(target_os = "macos"))]
    {
        Cow::Borrowed(path)
    }
    #[cfg(target_os = "macos")]
    {
        if path.extension() == Some(OsStr::new("app")) {
            Cow::Owned(path.join("Contents/MacOS/Unity"))
        } else {
            Cow::Borrowed(path)
        }
    }
}

/// Returns the path to application path
///
/// On macOS, this function converts an executable file path to .app path.
/// On other platforms, does nothing.
pub fn get_app_path(path: &Path) -> Option<&Path> {
    #[cfg(not(target_os = "macos"))]
    {
        Some(path)
    }
    #[cfg(target_os = "macos")]
    {
        if path.ends_with("Contents/MacOS/Unity") {
            Some(path.parent().unwrap().parent().unwrap().parent().unwrap())
        } else if path.extension() == Some(OsStr::new("app")) {
            Some(path)
        } else {
            // It looks the path is not path to executable nor app bundle so return none
            None
        }
    }
}

#[allow(dead_code)]
async fn headless_unity_hub(unity_hub_path: &OsStr, args: &[&OsStr]) -> io::Result<Output> {
    let args = {
        let mut vec = Vec::with_capacity(args.len() + 2);
        if !cfg!(target_os = "linux") {
            vec.push("--".as_ref());
        }
        vec.push("--headless".as_ref());
        vec.extend_from_slice(args);
        vec
    };

    Command::new(unity_hub_path).args(args).output().await
}

pub async fn load_unity_by_calling_unity_hub(
    unity_hub_path: &OsStr,
) -> io::Result<Vec<(UnityVersion, PathBuf)>> {
    let output = headless_unity_hub(unity_hub_path, &["editors".as_ref(), "-i".as_ref()]).await?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Unity Hub failed to get installed unity versions",
        ));
    }

    let stdout = from_utf8(&output.stdout)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid utf8 from unity hub"))?;

    let mut result = Vec::new();

    for x in stdout.lines() {
        let Some((version_and_arch, path)) = x.split_once("installed at") else {
            continue;
        };
        let version = version_and_arch
            .split_once(' ')
            .map(|(v, _)| v)
            .unwrap_or(version_and_arch);
        let Some(version) = UnityVersion::parse(version) else {
            continue;
        };

        result.push((version, PathBuf::from(path.trim())));
    }

    Ok(result)
}

pub async fn get_unity_from_unity_hub(
    _unity_hub_path: &OsStr,
) -> io::Result<Vec<(UnityVersion, PathBuf)>> {
    let mut result = Vec::new();

    for x in load_unity_by_loading_unity_hub_files().await? {
        result.push((x.version, get_executable_path(&x.path).into_owned()));
    }

    Ok(result)
}

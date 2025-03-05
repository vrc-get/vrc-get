mod find_unity_from_unity_hub_logic;
mod os;

use crate::io;
use crate::version::UnityVersion;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Output;
use tokio::process::Command;

pub use find_unity_from_unity_hub_logic::find_available_editors;
pub use os::load_unity_version;

/// Returns the path to executable file
///
/// On macOS, this function expects Path to .app file and returns binary file.
/// On other platforms, does nothing
pub fn get_executable_path(path: &Path) -> Cow<Path> {
    #[cfg(not(target_os = "macos"))]
    {
        Cow::Borrowed(path)
    }
    #[cfg(target_os = "macos")]
    {
        Cow::Owned(path.join("Contents/MacOS/Unity"))
    }
}

/// Returns the path to application path
///
/// On macOS, this function converts an executable file path to .app path.
/// On other platforms, does nothing.
pub fn get_app_path(path: &Path) -> &Path {
    #[cfg(not(target_os = "macos"))]
    {
        path
    }
    #[cfg(target_os = "macos")]
    {
        if path.ends_with("Contents/MacOS/Unity") {
            path.parent().unwrap().parent().unwrap().parent().unwrap()
        } else {
            // Fallback to normal path.
            path
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

pub async fn get_unity_from_unity_hub(
    _unity_hub_path: &OsStr,
) -> io::Result<Vec<(UnityVersion, PathBuf)>> {
    let mut result = Vec::new();

    for x in find_available_editors().await? {
        result.push((x.version, get_executable_path(&x.path).into_owned()));
    }

    Ok(result)
}

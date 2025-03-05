mod find_unity_from_unity_hub_logic;
mod os;

use crate::io;
use crate::version::UnityVersion;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Output;
use tokio::process::Command;

pub use find_unity_from_unity_hub_logic::find_available_editors;

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

    #[cfg(not(target_os = "macos"))]
    fn unity_path(original: PathBuf) -> PathBuf {
        original
    }

    #[cfg(target_os = "macos")]
    fn unity_path(original: PathBuf) -> PathBuf {
        // on macos, unity hub returns path to app bundle folder, not the executable
        original.join("Contents/MacOS/Unity")
    }

    for x in find_available_editors().await? {
        result.push((x.version, unity_path(x.path)));
    }

    Ok(result)
}

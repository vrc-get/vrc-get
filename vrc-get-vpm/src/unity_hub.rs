use crate::io;
use crate::version::UnityVersion;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Output;
use std::str::from_utf8;
use tokio::process::Command;

async fn headless_unity_hub(unity_hub_path: &OsStr, args: &[&OsStr]) -> io::Result<Output> {
    let args = {
        let mut vec = Vec::with_capacity(args.len() + 2);
        if cfg!(target_os = "linux") && unity_hub_path.to_str() == Some("flatpak") {
            vec.push("run".as_ref());
            vec.push("com.unity.UnityHub".as_ref());
            vec.push("--".as_ref());
        }
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

    #[cfg(not(target_os = "macos"))]
    fn unity_path(original: &str) -> PathBuf {
        PathBuf::from(original)
    }

    #[cfg(target_os = "macos")]
    fn unity_path(original: &str) -> PathBuf {
        // on macos, unity hub returns path to app bundle folder, not the executable
        PathBuf::from(original).join("Contents/MacOS/Unity")
    }

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

        result.push((version, unity_path(path.trim())));
    }

    Ok(result)
}

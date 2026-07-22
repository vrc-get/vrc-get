use anyhow::Context;
use std::path::Path;
use std::process::Command;

/// Downloads a file from `url` to `dest` with curl on unix and `Invoke-WebRequest` on windows.
pub fn download_file(url: &str, dest: &Path, what: &str) -> anyhow::Result<()> {
    static USER_AGENT: &str = "cargo-xtask of vrc-get (https://github.com/vrc-get/vrc-get)";

    let status = if cfg!(windows) {
        println!("downloading {url} ...");
        Command::new("powershell")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .args(["-ExecutionPolicy", "Bypass"])
            .arg("-Command")
            .arg("& { $ProgressPreference = 'SilentlyContinue'; Invoke-WebRequest -Uri $env:__xtask_url -OutFile $env:__xtask_dest -UserAgent $env:__xtask_user_agent }")
            .env("__xtask_url", url)
            .env("__xtask_dest", dest)
            .env("__xtask_user_agent", USER_AGENT)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .with_context(|| format!("{what}: downloading {url}: executing Invoke-WebRequest"))?
    } else if cfg!(unix) {
        println!("downloading {url} ...");
        Command::new("curl")
            .arg("-L")
            .arg("--fail-with-body")
            .args(["--user-agent", USER_AGENT])
            .arg("-o")
            .arg(dest)
            .arg(url)
            .status()
            .with_context(|| format!("{what}: downloading {url}: executing curl"))?
    } else {
        return Err(anyhow::anyhow!(
            "{what}: unsupported platform for downloading file"
        ));
    };
    if !status.success() {
        return Err(anyhow::anyhow!("{what}: downloading {url}: failed"));
    }
    Ok(())
}

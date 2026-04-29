use crate::utils::command::CommandExt;
use anyhow::bail;
use std::process::Command;
use std::sync::OnceLock;

pub fn rustc_host_triple() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();

    let mut cmd = Command::new("rustc");
    cmd.arg("-vV");
    let output = cmd
        .run_capture_checked("querying rustc host")
        .expect("failed to query");
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("host: ") {
            return CACHE.get_or_init(|| rest.trim().to_owned());
        }
    }
    panic!("failed to parse rustc host triple")
}

pub fn rustc_sysroot() -> anyhow::Result<&'static str> {
    static CACHE: OnceLock<String> = OnceLock::new();

    let mut cmd = Command::new("rustc");
    cmd.arg("--print").arg("sysroot");
    let output = cmd.run_capture_checked("querying rustc sysroot")?;
    let sysroot = output.trim();
    if sysroot.is_empty() {
        bail!("rustc --print sysroot returned empty output");
    }

    Ok(CACHE.get_or_init(|| sysroot.to_owned()))
}

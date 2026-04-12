#![allow(dead_code)]

use crate::utils;
use crate::utils::rustc::rustc_host_triple;
use anyhow::Context;
use std::io::IoSlice;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::{fs, io};

pub mod cargo;
pub mod command;
pub mod ds_store;
pub mod rustc;
pub mod tar;

pub fn ureq() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();

    AGENT.get_or_init(|| {
        ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .user_agent("cargo-xtask of vrc-get (https://github.com/vrc-get/vrc-get)")
                .build(),
        )
    })
}

pub trait MayOption<T> {
    fn into_option(self) -> Option<T>;
}

impl<T> MayOption<T> for Option<T> {
    fn into_option(self) -> Option<T> {
        self
    }
}

impl<T> MayOption<T> for T {
    fn into_option(self) -> Option<T> {
        Some(self)
    }
}

pub fn build_target<'a>(target: impl MayOption<&'a str>) -> &'a str {
    let host_triple = rustc_host_triple();
    target.into_option().unwrap_or(host_triple)
}

pub fn build_dir<'a>(target: impl MayOption<&'a str>, profile: &str) -> PathBuf {
    let metadata = cargo::cargo_metadata();
    let target_dir = metadata.target_directory.as_std_path();

    match target.into_option() {
        None => target_dir.join(profile),
        Some(target) => target_dir.join(target).join(profile),
    }
}

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("profile_select")
        .args(["release", "profile"])
))]
pub struct BuildProfile {
    #[arg(long)]
    release: bool,

    #[arg(long)]
    profile: Option<String>,
}

impl BuildProfile {
    pub fn with_default<'a>(&'a self, default: &'a str) -> &'a str {
        if self.release {
            "release"
        } else if let Some(profile) = &self.profile {
            profile
        } else {
            default
        }
    }

    pub fn name(&self) -> &str {
        self.with_default("dev")
    }
}

/// Make a file executable (mode 755).
#[cfg(unix)]
pub fn make_executable(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .with_context(|| format!("chmod 755 {}", path.display()))
}

#[cfg(not(unix))]
pub fn make_executable(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

pub fn estimated_dir_size(path: &Path) -> Option<u64> {
    let mut total = 0u64;
    for entry in fs::read_dir(path).ok()? {
        let Ok(entry) = entry else {
            continue;
        };
        if let Ok(meta) = entry.metadata() {
            if meta.is_dir() {
                total += estimated_dir_size(&entry.path()).unwrap_or(0);
            } else {
                total += meta.len();
            }
        }
    }
    Some(total)
}

pub struct CountingIo<T> {
    count: u64,
    inner: T,
}

impl<T> CountingIo<T> {
    pub fn new(inner: T) -> Self {
        Self { count: 0, inner }
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: io::Write> io::Write for CountingIo<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf).inspect(|&x| {
            self.count += x as u64;
        })
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.inner.write_vectored(bufs).inspect(|&x| {
            self.count += x as u64;
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Download a file from `url` to `dest`, skipping if the file already exists.
pub fn download_file_cached(url: &str, dest: &Path, what: &str) -> anyhow::Result<()> {
    if dest.is_file() {
        println!("cached: {}", dest.display());
        return Ok(());
    }
    fs::create_dir_all(dest.parent().unwrap())?;

    let mut response = utils::ureq()
        .get(url)
        .call()
        .with_context(|| format!("{what}: downloading {url}"))?;

    std::io::copy(
        &mut response.body_mut().as_reader(),
        &mut fs::File::create(dest)
            .with_context(|| format!("{what}: creating {}", dest.display()))?,
    )
    .with_context(|| format!("{what}: saving {url}"))?;

    println!("downloaded: {}", dest.display());
    Ok(())
}

#[allow(clippy::iter_nth_zero)] // actually we're accessing 0th element
pub fn target_arch(target: &str) -> &str {
    target.split('-').nth(0).unwrap()
}

pub fn target_vendor(target: &str) -> &str {
    target.split('-').nth(1).unwrap_or("unknown")
}

pub fn target_os(target: &str) -> &str {
    target.split('-').nth(2).unwrap_or("none")
}

pub fn target_abi(target: &str) -> &str {
    target.split('-').nth(3).unwrap_or("none")
}

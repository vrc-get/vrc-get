use crate::utils::rustc::rustc_host_triple;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

mod linux;
mod macos;

/// Bundles the ALCOM application for the target platform.
///
/// This reimplements the tauri bundler functionality so we do not need to depend on
/// `tauri bundle` / `tauri-apps/tauri-action` for creating distributable packages.
///
/// Outputs (relative to the profile build directory, e.g. `target/<triple>/release/`):
///
/// **macOS**
/// - `bundle/macos/ALCOM.app`                       – application bundle
/// - `bundle/macos/ALCOM.app.tar.gz`               – tarball used by the updater
/// - `bundle/dmg/ALCOM_<version>_universal.dmg`    – disk image for distribution
///
/// **Linux**
/// - `bundle/appimage/ALCOM_<version>_amd64.AppImage`                  – portable image
/// - `bundle/appimage/ALCOM_<version>_amd64.AppImage.tar.gz`           – tarball for updater
/// - `bundle/deb/ALCOM_<version>_amd64.deb`                            – Debian package
/// - `bundle/rpm/ALCOM-<version>-1.x86_64.rpm`                         – RPM package
#[derive(clap::Parser)]
pub(super) struct Command {
    /// Target triple (e.g. `universal-apple-darwin`, `x86_64-unknown-linux-gnu`).
    ///
    /// Defaults to the host triple.
    #[arg(long)]
    target: Option<String>,

    /// Build profile (default: `release`).
    #[arg(long, default_value = "release")]
    profile: String,
}

impl crate::Command for Command {
    fn run(self) -> Result<i32> {
        let metadata = crate::utils::cargo::cargo_metadata();
        let workspace_root = metadata.workspace_root.as_std_path();
        let target_dir = metadata.target_directory.as_std_path();

        let host_triple = rustc_host_triple()?;
        let target_triple = self.target.as_deref().unwrap_or(host_triple);

        let build_dir = if target_triple == host_triple {
            target_dir.join(&self.profile)
        } else {
            target_dir.join(target_triple).join(&self.profile)
        };

        let gui_dir = workspace_root.join("vrc-get-gui");

        let config = BundleConfig::load(&gui_dir, workspace_root)?;

        let bundle_dir = build_dir.join("bundle");

        let ctx = BundleContext {
            workspace_root,
            gui_dir: &gui_dir,
            build_dir: &build_dir,
            bundle_dir: &bundle_dir,
            target_triple,
            config: &config,
        };

        if target_triple.contains("apple") {
            macos::bundle(&ctx)?;
        } else if target_triple.contains("linux") {
            linux::bundle(&ctx)?;
        } else if target_triple.contains("windows") {
            // Windows bundling is handled by the build-alcom-installer command.
            println!("Windows bundling is handled by build-alcom-installer.");
        } else {
            bail!("unsupported target triple: {target_triple}");
        }

        Ok(0)
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Parsed subset of `Tauri.toml` and `Cargo.toml` that the bundler needs.
pub(crate) struct BundleConfig {
    pub product_name: String,
    pub identifier: String,
    pub version: String,
    pub short_description: String,
    pub long_description: String,
    pub copyright: String,
    pub category: String,
    pub publisher: String,
    /// Paths of icon files listed in `[bundle] icon`, relative to `gui_dir`.
    pub icons: Vec<String>,
}

impl BundleConfig {
    fn load(gui_dir: &Path, workspace_root: &Path) -> Result<Self> {
        // --- parse Tauri.toml ---
        let tauri_toml_path = gui_dir.join("Tauri.toml");
        let tauri_toml_src = fs::read_to_string(&tauri_toml_path)
            .with_context(|| format!("reading {}", tauri_toml_path.display()))?;
        let tauri_toml: TauriToml = toml::from_str(&tauri_toml_src)
            .with_context(|| format!("parsing {}", tauri_toml_path.display()))?;

        // --- read version from Cargo.toml (workspace metadata) ---
        let cargo_toml_path = gui_dir.join("Cargo.toml");
        let cargo_toml_src = fs::read_to_string(&cargo_toml_path)
            .with_context(|| format!("reading {}", cargo_toml_path.display()))?;
        let cargo_toml: CargoToml = toml::from_str(&cargo_toml_src)
            .with_context(|| format!("parsing {}", cargo_toml_path.display()))?;

        let _ = workspace_root; // may be used in the future

        Ok(BundleConfig {
            product_name: tauri_toml.product_name,
            identifier: tauri_toml.identifier,
            version: cargo_toml.package.version,
            short_description: tauri_toml.bundle.short_description.unwrap_or_default(),
            long_description: tauri_toml.bundle.long_description.unwrap_or_default(),
            copyright: tauri_toml.bundle.copyright.unwrap_or_default(),
            category: tauri_toml.bundle.category.unwrap_or_default(),
            publisher: tauri_toml.bundle.publisher.unwrap_or_default(),
            icons: tauri_toml.bundle.icon.unwrap_or_default(),
        })
    }
}

/// Shared context passed to every platform bundler.
pub(crate) struct BundleContext<'a> {
    #[allow(dead_code)]
    pub workspace_root: &'a Path,
    pub gui_dir: &'a Path,
    pub build_dir: &'a Path,
    pub bundle_dir: &'a Path,
    pub target_triple: &'a str,
    pub config: &'a BundleConfig,
}

impl BundleContext<'_> {
    /// Binary name without extension (e.g. `ALCOM`).
    pub fn binary_name(&self) -> &str {
        &self.config.product_name
    }

    /// Path to the compiled binary in the build directory.
    pub fn binary_path(&self) -> PathBuf {
        if self.target_triple.contains("windows") {
            self.build_dir.join(format!("{}.exe", self.binary_name()))
        } else {
            self.build_dir.join(self.binary_name())
        }
    }

    /// Resolved path of an icon file listed in `Tauri.toml`.
    pub fn icon_path(&self, relative: &str) -> PathBuf {
        self.gui_dir.join(relative)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a `.tar.gz` archive at `out_path` containing a single file `src`
/// whose name inside the archive is `archive_name`.
pub(crate) fn create_tar_gz(src: &Path, archive_name: &str, out_path: &Path) -> Result<()> {
    use flate2::Compression;
    use flate2::write::GzEncoder;

    fs::create_dir_all(out_path.parent().unwrap())?;

    let file =
        fs::File::create(out_path).with_context(|| format!("creating {}", out_path.display()))?;
    let gz = GzEncoder::new(file, Compression::best());
    let mut builder = tar::Builder::new(gz);
    builder.follow_symlinks(false);

    if src.is_dir() {
        builder
            .append_dir_all(archive_name, src)
            .with_context(|| format!("appending dir {}", src.display()))?;
    } else {
        builder
            .append_path_with_name(src, archive_name)
            .with_context(|| format!("appending file {}", src.display()))?;
    }

    let gz = builder.into_inner().context("finishing tar archive")?;
    gz.finish().context("finishing gzip stream")?;

    println!("created: {}", out_path.display());
    Ok(())
}

/// Run a command and check that it exits successfully.
pub(crate) fn run_cmd(cmd: &mut ProcessCommand, what: &str) -> Result<()> {
    use crate::utils::command::CommandExt;
    cmd.run_checked(what)
}

/// Download a file from `url` to `dest`, skipping if the file already exists.
pub(crate) fn download_file_cached(url: &str, dest: &Path, what: &str) -> Result<()> {
    if dest.is_file() {
        println!("cached: {}", dest.display());
        return Ok(());
    }
    fs::create_dir_all(dest.parent().unwrap())?;

    let mut response = crate::utils::ureq()
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

// ---------------------------------------------------------------------------
// Tauri.toml serde types (only the fields we need)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TauriToml {
    product_name: String,
    identifier: String,
    bundle: BundleSection,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleSection {
    short_description: Option<String>,
    long_description: Option<String>,
    copyright: Option<String>,
    category: Option<String>,
    publisher: Option<String>,
    icon: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Cargo.toml serde types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CargoToml {
    package: CargoPackage,
}

#[derive(Deserialize)]
struct CargoPackage {
    version: String,
}

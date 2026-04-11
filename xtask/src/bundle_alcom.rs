use crate::utils::{self, build_dir, build_target};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

mod app;
mod appimage;
mod deb;
mod dmg;
mod linux;
mod rpm;

/// Individual bundle artifact that can be produced.
///
/// Pass one or more values to `--bundles` to produce only those artifacts.
/// If `--bundles` is not specified, all artifacts for the target platform are produced.
///
/// **macOS** artifacts:
/// - `app` — `ALCOM.app` application bundle
/// - `dmg` — `ALCOM_<version>_<arch>.dmg` disk image
/// - `app-updater` — `ALCOM.app.tar.gz` updater payload
///
/// **Linux** artifacts:
/// - `app-image` — `ALCOM_<version>_<arch>.AppImage`
/// - `app-image-updater` — `ALCOM_<version>_<arch>.AppImage.tar.gz` updater payload
/// - `deb` — `ALCOM_<version>_<arch>.deb` Debian package
/// - `rpm` — `ALCOM-<version>-1.<arch>.rpm` RPM package
#[derive(clap::ValueEnum, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BundleKind {
    // --- macOS ---
    /// ALCOM.app bundle
    App,
    /// Disk image (requires ALCOM.app to already exist in bundle dir)
    Dmg,
    /// ALCOM.app.tar.gz updater payload (requires ALCOM.app to already exist)
    AppUpdater,
    // --- Linux ---
    /// AppImage portable image
    AppImage,
    /// AppImage.tar.gz updater payload (requires AppImage to already exist)
    AppImageUpdater,
    /// Debian package
    Deb,
    /// RPM package
    Rpm,
}

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

    #[command(flatten)]
    profile: utils::BuildProfile,

    /// Specific bundle artifacts to produce (comma-separated or repeated).
    ///
    /// When not specified, all artifacts for the target platform are produced.
    /// Use this to split the bundling process — e.g. produce only `app` first,
    /// then sign it, then produce `dmg` and `app-updater`.
    #[arg(long, value_delimiter = ',')]
    bundles: Vec<BundleKind>,
}

impl crate::Command for Command {
    fn run(self) -> Result<i32> {
        let metadata = utils::cargo::cargo_metadata();
        let workspace_root = metadata.workspace_root.as_std_path();

        let target_truple = build_target(self.target.as_deref());
        let build_dir = build_dir(self.target.as_deref(), self.profile.name());

        let gui_dir = workspace_root.join("vrc-get-gui");

        let config = BundleConfig::load(&gui_dir, workspace_root)?;

        let bundle_dir = build_dir.join("bundle");

        let bundles = default_bundles_if_empty(&self.bundles, target_truple)?;

        let ctx = BundleContext {
            workspace_root,
            gui_dir: &gui_dir,
            host_build_dir: metadata.target_directory.as_std_path(),
            build_dir: &build_dir,
            bundle_dir: &bundle_dir,
            target_truple,
            config: &config,
        };

        if bundles.contains(&BundleKind::App) {
            app::create_app_bundle(&ctx)?;
        }

        if bundles.contains(&BundleKind::AppUpdater) {
            app::create_app_tar_gz(&ctx)?;
        }

        if bundles.contains(&BundleKind::Dmg) {
            dmg::create_dmg(&ctx)?;
        }

        if bundles.contains(&BundleKind::AppImage) {
            appimage::create_appimage(&ctx)?;
        }

        if bundles.contains(&BundleKind::AppImageUpdater) {
            appimage::create_appimage_tar_gz(&ctx)?;
        }

        if bundles.contains(&BundleKind::Deb) {
            deb::create_deb(&ctx)?;
        }

        if bundles.contains(&BundleKind::Rpm) {
            rpm::create_rpm(&ctx)?;
        }

        Ok(0)
    }
}

fn default_bundles_if_empty<'a>(
    bundles: &'a [BundleKind],
    target_tuple: &str,
) -> Result<&'a [BundleKind]> {
    if bundles.is_empty() {
        if target_tuple.contains("apple") {
            Ok(&[BundleKind::App, BundleKind::AppUpdater, BundleKind::Dmg])
        } else if target_tuple.contains("linux") {
            Ok(&[
                BundleKind::AppImage,
                BundleKind::AppImageUpdater,
                BundleKind::Deb,
                BundleKind::Rpm,
            ])
        } else if target_tuple.contains("windows") {
            // Windows bundling is handled by the build-alcom-installer command.
            println!("Windows bundling is handled by build-alcom-installer.");
            Ok(&[])
        } else {
            bail!("unsupported target triple: {target_tuple}");
        }
    } else {
        Ok(bundles)
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
        })
    }
}

/// Shared context passed to every platform bundler.
pub(crate) struct BundleContext<'a> {
    #[allow(dead_code)]
    pub workspace_root: &'a Path,
    pub gui_dir: &'a Path,
    pub host_build_dir: &'a Path,
    pub build_dir: &'a Path,
    pub bundle_dir: &'a Path,
    pub target_truple: &'a str,
    pub config: &'a BundleConfig,
}

impl BundleContext<'_> {
    /// Binary name without extension (e.g. `ALCOM`).
    pub fn binary_name(&self) -> &str {
        &self.config.product_name
    }

    /// Path to the compiled binary in the build directory.
    pub fn binary_path(&self) -> PathBuf {
        if self.target_truple.contains("windows") {
            self.build_dir.join(format!("{}.exe", self.binary_name()))
        } else {
            self.build_dir.join(self.binary_name())
        }
    }

    /// Resolved path of an icon file listed in `Tauri.toml`.
    pub fn icon_path(&self, size: &str) -> PathBuf {
        self.gui_dir.join("icons").join(size).with_extension("png")
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

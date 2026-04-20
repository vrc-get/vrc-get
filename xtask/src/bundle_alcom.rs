use crate::utils::{self, build_dir, build_target, target_os};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

mod app;
mod appimage;
mod deb;
mod dmg;
mod linux;
mod rpm;
mod setup_exe;

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
    #[value(name = "appimage")]
    AppImage,
    /// AppImage.tar.gz updater payload (requires AppImage to already exist)
    #[value(name = "appimage-updater")]
    AppImageUpdater,
    /// Package manager independent buildroot for external package managers
    ///
    /// Unlike dmg depends on app, deb/rpm doesn't depend on this bundle.
    Buildroot,
    /// Debian package
    Deb,
    /// RPM package
    Rpm,
    /// Windows setup.exe
    SetupExe,
    /// Windows setup.exe for updater
    ExeUpdater,
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

    /// Output directory for buildroot
    ///
    /// Only valid for buildroot bundle
    #[arg(long)]
    buildroot: Option<PathBuf>,
}

impl crate::Command for Command {
    fn run(self) -> Result<i32> {
        let ctx = BundleContext::new(self.target.as_deref(), self.profile.name())?;

        let bundles = self.bundles.as_slice();

        if bundles.is_empty() {
            println!("Note: no bundles are specified");
        }

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

        if bundles.contains(&BundleKind::Buildroot) {
            linux::create_install_build_root(&ctx, self.buildroot.as_deref())?;
        }

        if bundles.contains(&BundleKind::Deb) {
            deb::create_deb(&ctx)?;
        }

        if bundles.contains(&BundleKind::Rpm) {
            rpm::create_rpm(&ctx)?;
        }

        if bundles.contains(&BundleKind::SetupExe) {
            setup_exe::create_setup_exe(&ctx)?;
        }

        if bundles.contains(&BundleKind::ExeUpdater) {
            setup_exe::create_updater_exe(&ctx)?;
        }

        Ok(0)
    }
}

/// Shared context passed to every platform bundler.
pub(crate) struct BundleContext<'a> {
    #[allow(dead_code)]
    pub workspace_root: &'a Path,
    pub gui_dir: PathBuf,
    pub host_build_dir: &'a Path,
    pub build_dir: PathBuf,
    pub bundle_dir: PathBuf,
    pub target: Option<&'a str>,
    pub target_tuple: &'a str,
    pub profile: &'a str,
    version: String,
}

impl<'a> BundleContext<'a> {
    pub fn new(target: Option<&'a str>, profile: &'a str) -> Result<Self> {
        let metadata = utils::cargo::cargo_metadata();
        let workspace_root = metadata.workspace_root.as_std_path();

        let target_tuple = build_target(target);
        let build_dir = build_dir(target, profile);

        let gui_dir = workspace_root.join("vrc-get-gui");

        let version = (metadata.packages.iter())
            .find(|p| p.name == "vrc-get-gui")
            .context("finding vrc-get-gui")?
            .version
            .to_string();

        let bundle_dir = build_dir.join("bundle");

        Ok(BundleContext {
            workspace_root,
            gui_dir,
            host_build_dir: metadata.target_directory.as_std_path(),
            build_dir,
            bundle_dir,
            target,
            target_tuple,
            profile,
            version,
        })
    }

    pub fn version(&self) -> &str {
        self.version.as_str()
    }

    pub fn short_description(&self) -> &str {
        "ALCOM - Alternative Creator Companion"
    }

    pub fn long_description(&self) -> &str {
        "ALCOM is a fast and open-source alternative VCC (VRChat Creator Companion) written in rust and tauri."
    }

    /// Binary name without extension (e.g. `ALCOM`).
    pub fn binary_name(&self) -> &str {
        "ALCOM"
    }

    /// The human-readable product name.
    pub fn product_name(&self) -> &str {
        "ALCOM"
    }

    /// The machine-readable identifier of the product.
    ///
    /// This is named 'vrc-get-gui' for historical reasons.
    pub fn identifier(&self) -> &str {
        "com.anatawa12.vrc-get-gui"
    }

    /// The simplified copyright notice for this product
    pub fn copyright(&self) -> &str {
        "(c) anatawa12 and other contributors"
    }

    /// Path to the compiled binary in the build directory.
    pub fn binary_path(&self) -> PathBuf {
        if target_os(self.target_tuple) == "windows" {
            self.build_dir.join(format!("{}.exe", self.binary_name()))
        } else {
            self.build_dir.join(self.binary_name())
        }
    }

    /// Resolved path of an icon file
    pub fn icon_path(&self, name: &str) -> PathBuf {
        let mut pathbuf = self.gui_dir.join("icons").join(name);
        if pathbuf.extension().is_none() {
            pathbuf.set_extension("png");
        }
        pathbuf
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

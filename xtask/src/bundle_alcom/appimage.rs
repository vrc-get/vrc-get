use super::{BundleContext, create_tar_gz};
use crate::bundle_alcom::linux::LINUX_ICON_RESOLUTIONS;
use crate::utils::command::CommandExt;
use crate::utils::{download_file_cached, make_executable, target_arch};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

// appimage versions we currently used
const APPIMAGETOOL_VERSION: &str = "13";
const APPIMAGETOOL_URL: &str =
    "https://github.com/AppImage/AppImageKit/releases/download/13/appimagetool-x86_64.AppImage";

fn appimage_name(ctx: &BundleContext<'_>) -> Result<String> {
    Ok(format!(
        "ALCOM_{}_{arch}.AppImage",
        ctx.config.version,
        arch = target_arch(ctx.target_truple)
    ))
}

/// Builds the AppImage and returns the path to the created file.
pub fn create_appimage(ctx: &BundleContext<'_>) -> Result<PathBuf> {
    let appdir = ctx.bundle_dir.join("appimage").join("ALCOM.AppDir");
    prepare_appdir(ctx, &appdir)?;

    let tool = ensure_appimagetool(ctx)?;
    make_executable(&tool)?;

    let name = format!(
        "ALCOM_{}_{}.AppImage",
        ctx.config.version,
        target_arch(ctx.target_truple)
    );
    let out_dir = ctx.bundle_dir.join("appimage");
    fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(&name);

    // appimagetool requires ARCH to be set for non-native builds.

    ProcessCommand::new(&tool)
        .arg(&appdir)
        .arg(&out_path)
        .env("ARCH", target_arch(ctx.target_truple))
        // Avoid FUSE requirement when running appimagetool (which is itself an AppImage)
        // in environments where FUSE is not available (e.g. GitHub Actions).
        .env("APPIMAGE_EXTRACT_AND_RUN", "1")
        .run_checked("creating AppImage")?;

    println!("created: {}", out_path.display());
    Ok(out_path)
}

pub fn create_appimage_tar_gz(ctx: &BundleContext<'_>) -> Result<()> {
    let appimage = ctx.bundle_dir.join("appimage").join(appimage_name(ctx)?);

    let archive_name = format!(
        "{}.tar.gz",
        appimage.file_name().and_then(|n| n.to_str()).unwrap()
    );
    let out = appimage.parent().unwrap().join(&archive_name);
    let inner_name = appimage.file_name().and_then(|n| n.to_str()).unwrap();

    create_tar_gz(&appimage, inner_name, &out)
}

/// Populate the AppDir structure.
fn prepare_appdir(ctx: &BundleContext<'_>, appdir: &Path) -> Result<()> {
    if appdir.exists() {
        fs::remove_dir_all(appdir)?;
    }

    let bin_dir = appdir.join("usr/bin");
    let share_apps = appdir.join("usr/share/applications");
    let icons_dir = appdir.join("usr/share/icons/hicolor");

    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&share_apps)?;

    // Binary
    let bin_name = ctx.binary_name();
    let bin_dst = bin_dir.join(bin_name);
    fs::copy(ctx.binary_path(), &bin_dst).context("copying binary to AppDir")?;
    make_executable(&bin_dst)?;

    // AppRun (wrapper that executes the binary)
    let apprun_path = appdir.join("AppRun");
    fs::write(
        &apprun_path,
        format!("#!/bin/sh\nexec \"$(dirname \"$0\")/usr/bin/{bin_name}\" \"$@\"\n"),
    )?;
    make_executable(&apprun_path)?;

    // Desktop file
    let exec = format!("usr/bin/{bin_name}");
    let desktop_content = crate::bundle_alcom::linux::render_desktop_file(ctx, &exec)?;
    let desktop_name = "alcom.desktop";
    fs::write(appdir.join(desktop_name), &desktop_content)?;
    fs::create_dir_all(&share_apps)?;
    fs::write(share_apps.join(desktop_name), &desktop_content)?;

    // Icons
    install_icons(ctx, &icons_dir)?;

    // Also copy the 128x128 icon as the top-level .DirIcon and alcom.png for appimagetool.
    let icon_128 = ctx.icon_path("128x128");
    fs::copy(&icon_128, appdir.join(".DirIcon"))?;
    fs::copy(&icon_128, appdir.join("alcom.png"))?;

    Ok(())
}
pub fn install_icons(ctx: &BundleContext<'_>, icons_base: &Path) -> Result<()> {
    for size in LINUX_ICON_RESOLUTIONS {
        let icon_dir = icons_base.join(size).join("apps");
        fs::create_dir_all(&icon_dir)?;
        fs::copy(ctx.icon_path(size), icon_dir.join("alcom.png"))
            .with_context(|| format!("copying icon {size}.png"))?;
    }
    Ok(())
}

/// Ensures `appimagetool` is available in the target cache directory.
fn ensure_appimagetool(ctx: &BundleContext<'_>) -> Result<PathBuf> {
    let cache_dir = ctx
        .host_build_dir
        .join("appimagetool")
        .join(APPIMAGETOOL_VERSION);
    let tool = cache_dir.join("appimagetool-x86_64.AppImage");

    download_file_cached(APPIMAGETOOL_URL, &tool, "downloading appimagetool")?;
    Ok(tool)
}

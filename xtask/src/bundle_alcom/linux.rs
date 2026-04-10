use super::{create_tar_gz, download_file_cached, run_cmd, BundleContext};
use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

/// `appimagetool` release to download when not already present.
const APPIMAGETOOL_VERSION: &str = "13";
const APPIMAGETOOL_URL: &str =
    "https://github.com/AppImage/AppImageKit/releases/download/13/appimagetool-x86_64.AppImage";

/// Create all Linux bundles: AppImage, `.AppImage.tar.gz`, `.deb`, `.rpm`.
pub fn bundle(ctx: &BundleContext<'_>) -> Result<()> {
    let appimage = create_appimage(ctx)?;
    create_appimage_tar_gz(ctx, &appimage)?;
    create_deb(ctx)?;
    create_rpm(ctx)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Debian/Ubuntu architecture string for the host (assumed x86_64 unless triple says aarch64).
fn deb_arch(triple: &str) -> &str {
    if triple.contains("aarch64") {
        "arm64"
    } else {
        "amd64"
    }
}

/// RPM architecture string.
fn rpm_arch(triple: &str) -> &str {
    if triple.contains("aarch64") {
        "aarch64"
    } else {
        "x86_64"
    }
}

/// Render the desktop file template from `alcom.desktop`.
///
/// The template uses `{{key}}` placeholders as in the tauri bundler.
fn render_desktop_file(ctx: &BundleContext<'_>, exec: &str) -> Result<String> {
    let template_path = ctx.gui_dir.join("alcom.desktop");
    let template = fs::read_to_string(&template_path)
        .with_context(|| format!("reading {}", template_path.display()))?;

    let categories = match ctx.config.category.as_str() {
        "DeveloperTool" => "Development;",
        "Game" => "Game;",
        "AudioVideo" => "AudioVideo;",
        other => other,
    };

    let result = template
        .replace("{{categories}}", categories)
        .replace("{{comment}}", &ctx.config.short_description)
        .replace("{{exec}}", exec)
        .replace("{{icon}}", "alcom")
        .replace("{{name}}", &ctx.config.product_name)
        // Handle conditional {{#if comment}} block (simplified: always include comment)
        .replace("{{#if comment}}", "")
        .replace("{{/if}}", "");

    Ok(result)
}

/// Make a file executable (mode 755).
#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .with_context(|| format!("chmod 755 {}", path.display()))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

/// Install png icons into a hicolor-layout directory.
fn install_icons(ctx: &BundleContext<'_>, icons_base: &Path) -> Result<()> {
    for icon_rel in ["icons/32x32.png", "icons/64x64.png", "icons/128x128.png"] {
        let src = ctx.icon_path(icon_rel);
        if !src.exists() {
            continue;
        }
        let filename = Path::new(icon_rel)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("128x128");
        let icon_dir = icons_base.join(filename).join("apps");
        fs::create_dir_all(&icon_dir)?;
        fs::copy(&src, icon_dir.join("alcom.png"))
            .with_context(|| format!("copying icon {} → alcom.png", src.display()))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AppImage
// ---------------------------------------------------------------------------

fn appimage_name(ctx: &BundleContext<'_>) -> String {
    let arch = if ctx.target_triple.contains("aarch64") {
        "aarch64"
    } else {
        "amd64"
    };
    format!("ALCOM_{}_{arch}.AppImage", ctx.config.version)
}

/// Builds the AppImage and returns the path to the created file.
fn create_appimage(ctx: &BundleContext<'_>) -> Result<PathBuf> {
    let appdir = ctx.bundle_dir.join("appimage").join("ALCOM.AppDir");
    prepare_appdir(ctx, &appdir)?;

    let tool = ensure_appimagetool(ctx)?;
    make_executable(&tool)?;

    let name = appimage_name(ctx);
    let out_dir = ctx.bundle_dir.join("appimage");
    fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(&name);

    // appimagetool requires ARCH to be set for non-native builds.
    let arch_env = if ctx.target_triple.contains("aarch64") {
        "aarch64"
    } else {
        "x86_64"
    };

    run_cmd(
        ProcessCommand::new(&tool)
            .arg(&appdir)
            .arg(&out_path)
            .env("ARCH", arch_env)
            // Avoid FUSE requirement when running appimagetool (which is itself an AppImage)
            // in environments where FUSE is not available (e.g. GitHub Actions).
            .env("APPIMAGE_EXTRACT_AND_RUN", "1"),
        "creating AppImage",
    )?;

    println!("created: {}", out_path.display());
    Ok(out_path)
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
    let desktop_content = render_desktop_file(ctx, &exec)?;
    let desktop_name = "alcom.desktop";
    fs::write(appdir.join(desktop_name), &desktop_content)?;
    fs::create_dir_all(&share_apps)?;
    fs::write(share_apps.join(desktop_name), &desktop_content)?;

    // Icons
    install_icons(ctx, &icons_dir)?;

    // Also copy the 128x128 icon as the top-level .DirIcon and alcom.png for appimagetool.
    let icon_128 = ctx.icon_path("icons/128x128.png");
    if icon_128.exists() {
        fs::copy(&icon_128, appdir.join(".DirIcon"))?;
        fs::copy(&icon_128, appdir.join("alcom.png"))?;
    }

    Ok(())
}

/// Ensures `appimagetool` is available in the target cache directory.
fn ensure_appimagetool(ctx: &BundleContext<'_>) -> Result<PathBuf> {
    let cache_dir = ctx
        .build_dir
        .parent()
        .unwrap() // target/<profile> -> target
        .join("appimagetool")
        .join(APPIMAGETOOL_VERSION);
    let tool = cache_dir.join("appimagetool-x86_64.AppImage");

    download_file_cached(APPIMAGETOOL_URL, &tool, "downloading appimagetool")?;
    Ok(tool)
}

// ---------------------------------------------------------------------------
// AppImage.tar.gz
// ---------------------------------------------------------------------------

fn create_appimage_tar_gz(_ctx: &BundleContext<'_>, appimage: &Path) -> Result<()> {
    let archive_name = format!(
        "{}.tar.gz",
        appimage
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("ALCOM.AppImage")
    );
    let out = appimage.parent().unwrap().join(&archive_name);
    let inner_name = appimage
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("ALCOM.AppImage");
    create_tar_gz(appimage, inner_name, &out)
}

// ---------------------------------------------------------------------------
// .deb — built purely in Rust using the `ar` crate
// ---------------------------------------------------------------------------

fn create_deb(ctx: &BundleContext<'_>) -> Result<()> {
    let arch = deb_arch(ctx.target_triple);
    let pkg_name = format!("ALCOM_{}_{arch}", ctx.config.version);
    let deb_stage = ctx.bundle_dir.join("deb").join(&pkg_name);

    if deb_stage.exists() {
        fs::remove_dir_all(&deb_stage)?;
    }

    // Directory layout for the data tarball.
    let usr_bin = deb_stage.join("usr/bin");
    let share_apps = deb_stage.join("usr/share/applications");
    let icons_base = deb_stage.join("usr/share/icons/hicolor");

    fs::create_dir_all(&usr_bin)?;
    fs::create_dir_all(&share_apps)?;

    // Binary.
    let bin_name_lower = ctx.binary_name().to_ascii_lowercase();
    let bin_dst = usr_bin.join(&bin_name_lower);
    fs::copy(ctx.binary_path(), &bin_dst).context("copying binary to deb")?;
    make_executable(&bin_dst)?;

    // Desktop file.
    let desktop_content = render_desktop_file(ctx, &format!("/usr/bin/{bin_name_lower}"))?;
    let desktop_file = format!("{bin_name_lower}.desktop");
    fs::write(share_apps.join(&desktop_file), &desktop_content)?;

    // Icons.
    install_icons(ctx, &icons_base)?;

    // Build control.tar.gz
    let installed_size = dir_size(&deb_stage).unwrap_or(0) / 1024;
    let control = format!(
        "Package: alcom\nVersion: {version}\nArchitecture: {arch}\nInstalled-Size: {installed_size}\nMaintainer: {publisher}\nDescription: {short_desc}\n{long_desc}\n",
        version = ctx.config.version,
        arch = arch,
        installed_size = installed_size,
        publisher = ctx.config.publisher,
        short_desc = ctx.config.short_description,
        long_desc = ctx
            .config
            .long_description
            .lines()
            .map(|l| format!(" {l}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let control_tar_gz = build_control_tar_gz(control.as_bytes())?;

    // Build data.tar.gz
    let data_tar_gz = build_data_tar_gz(&deb_stage)?;

    // Assemble .deb as an ar archive.
    let deb_dir = ctx.bundle_dir.join("deb");
    fs::create_dir_all(&deb_dir)?;
    let deb_name = format!("{pkg_name}.deb");
    let deb_out = deb_dir.join(&deb_name);

    {
        let deb_file = fs::File::create(&deb_out)
            .with_context(|| format!("creating {}", deb_out.display()))?;
        let mut builder = ar::Builder::new(deb_file);

        // debian-binary
        let debian_binary = b"2.0\n";
        let mut header = ar::Header::new(b"debian-binary".to_vec(), debian_binary.len() as u64);
        header.set_mode(0o100644);
        builder
            .append(&header, &mut debian_binary.as_slice())
            .context("appending debian-binary")?;

        // control.tar.gz
        let mut header = ar::Header::new(b"control.tar.gz".to_vec(), control_tar_gz.len() as u64);
        header.set_mode(0o100644);
        builder
            .append(&header, &mut control_tar_gz.as_slice())
            .context("appending control.tar.gz")?;

        // data.tar.gz
        let mut header = ar::Header::new(b"data.tar.gz".to_vec(), data_tar_gz.len() as u64);
        header.set_mode(0o100644);
        builder
            .append(&header, &mut data_tar_gz.as_slice())
            .context("appending data.tar.gz")?;
    }

    println!("created: {}", deb_out.display());
    Ok(())
}

/// Build a `control.tar.gz` in memory from the given control file bytes.
fn build_control_tar_gz(control: &[u8]) -> Result<Vec<u8>> {
    use flate2::write::GzEncoder;
    use flate2::Compression;

    let mut out = Vec::new();
    let gz = GzEncoder::new(&mut out, Compression::best());
    let mut builder = tar::Builder::new(gz);

    let mut header = tar::Header::new_gnu();
    header.set_size(control.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "control", control)
        .context("appending control file")?;

    let gz = builder.into_inner().context("finishing control tar")?;
    gz.finish().context("finishing control gzip")?;

    Ok(out)
}

/// Build a `data.tar.gz` from all files under `stage_dir` in memory.
fn build_data_tar_gz(stage_dir: &Path) -> Result<Vec<u8>> {
    use flate2::write::GzEncoder;
    use flate2::Compression;

    let mut out = Vec::new();
    let gz = GzEncoder::new(&mut out, Compression::best());
    let mut builder = tar::Builder::new(gz);
    builder.follow_symlinks(false);

    // Append all files from the staging directory.
    append_dir_to_tar(&mut builder, stage_dir, stage_dir)?;

    let gz = builder.into_inner().context("finishing data tar")?;
    gz.finish().context("finishing data gzip")?;

    Ok(out)
}

fn append_dir_to_tar<W: Write>(
    builder: &mut tar::Builder<W>,
    root: &Path,
    dir: &Path,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap();
        let tar_path = format!("./{}", rel.display());

        if path.is_dir() {
            let mut header = tar::Header::new_gnu();
            header.set_mode(0o755);
            header.set_size(0);
            header.set_entry_type(tar::EntryType::Directory);
            header.set_cksum();
            builder
                .append_data(&mut header, &tar_path, &mut std::io::empty())
                .context("appending directory entry")?;
            append_dir_to_tar(builder, root, &path)?;
        } else {
            let metadata = fs::metadata(&path)?;
            let mode = {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    metadata.mode()
                }
                #[cfg(not(unix))]
                {
                    let _ = metadata;
                    0o644u32
                }
            };
            let mut header = tar::Header::new_gnu();
            header.set_mode(mode);
            header.set_size(
                fs::metadata(&path)
                    .with_context(|| format!("stat {}", path.display()))?
                    .len(),
            );
            header.set_cksum();
            let mut file = fs::File::open(&path)?;
            builder
                .append_data(&mut header, &tar_path, &mut file)
                .with_context(|| format!("appending {}", path.display()))?;
        }
    }
    Ok(())
}

fn dir_size(path: &Path) -> Option<u64> {
    let mut total = 0u64;
    for entry in fs::read_dir(path).ok()? {
        let entry = entry.ok()?;
        let meta = entry.metadata().ok()?;
        if meta.is_dir() {
            total += dir_size(&entry.path()).unwrap_or(0);
        } else {
            total += meta.len();
        }
    }
    Some(total)
}

// ---------------------------------------------------------------------------
// .rpm — built using the `rpm` crate (no external rpmbuild needed)
// ---------------------------------------------------------------------------

fn create_rpm(ctx: &BundleContext<'_>) -> Result<()> {
    let arch = rpm_arch(ctx.target_triple);
    let rpm_name = format!("ALCOM-{}-1.{arch}.rpm", ctx.config.version);
    let rpm_dir = ctx.bundle_dir.join("rpm");
    fs::create_dir_all(&rpm_dir)?;
    let rpm_out = rpm_dir.join(&rpm_name);

    let bin_name_lower = ctx.binary_name().to_ascii_lowercase();

    // Write desktop file to a temp location so rpm::PackageBuilder can read it from disk.
    let desktop_content = render_desktop_file(ctx, &format!("/usr/bin/{bin_name_lower}"))?;
    let desktop_tmp = rpm_dir.join(format!("{bin_name_lower}.desktop.tmp"));
    fs::write(&desktop_tmp, &desktop_content)?;

    let mut builder = rpm::PackageBuilder::new(
        "alcom",
        &ctx.config.version,
        "MIT",
        arch,
        &ctx.config.short_description,
    )
    .release("1")
    .description(&ctx.config.long_description);

    // Binary.
    builder = builder
        .with_file(
            ctx.binary_path(),
            rpm::FileOptions::new(format!("/usr/bin/{bin_name_lower}")).mode(0o755),
        )
        .context("adding binary to rpm")?;

    // Desktop file.
    builder = builder
        .with_file(
            &desktop_tmp,
            rpm::FileOptions::new(format!("/usr/share/applications/{bin_name_lower}.desktop")),
        )
        .context("adding desktop file to rpm")?;

    // Icons.
    for icon_rel in ["icons/32x32.png", "icons/64x64.png", "icons/128x128.png"] {
        let src = ctx.icon_path(icon_rel);
        if !src.exists() {
            continue;
        }
        let filename = Path::new(icon_rel)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("128x128");
        let install_path = format!("/usr/share/icons/hicolor/{filename}/apps/alcom.png");
        builder = builder
            .with_file(&src, rpm::FileOptions::new(install_path))
            .with_context(|| format!("adding icon {icon_rel} to rpm"))?;
    }

    let pkg = builder.build().context("building rpm package")?;
    pkg.write_file(&rpm_out)
        .with_context(|| format!("writing {}", rpm_out.display()))?;

    // Clean up temp desktop file.
    let _ = fs::remove_file(&desktop_tmp);

    println!("created: {}", rpm_out.display());
    Ok(())
}

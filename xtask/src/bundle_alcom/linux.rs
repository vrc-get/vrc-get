use super::{BundleContext, create_tar_gz, download_file_cached, run_cmd};
use anyhow::{Context, Result};
use std::fs;
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
    if triple.contains("aarch64") { "arm64" } else { "amd64" }
}

/// RPM architecture string.
fn rpm_arch(triple: &str) -> &str {
    if triple.contains("aarch64") { "aarch64" } else { "x86_64" }
}

/// Copy `src` to `dst`, creating parent directories as needed.
#[allow(dead_code)]
fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst.parent().unwrap())
        .with_context(|| format!("creating parent of {}", dst.display()))?;
    fs::copy(src, dst)
        .with_context(|| format!("copying {} → {}", src.display(), dst.display()))?;
    Ok(())
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
    fs::copy(ctx.binary_path(), &bin_dst)
        .context("copying binary to AppDir")?;
    make_executable(&bin_dst)?;

    // AppRun (wrapper that executes the binary)
    let apprun_path = appdir.join("AppRun");
    fs::write(
        &apprun_path,
        format!(
            "#!/bin/sh\nexec \"$(dirname \"$0\")/usr/bin/{bin_name}\" \"$@\"\n"
        ),
    )?;
    make_executable(&apprun_path)?;

    // Desktop file
    let exec = format!("usr/bin/{bin_name}");
    let desktop_content = render_desktop_file(ctx, &exec)?;
    let desktop_name = "alcom.desktop";
    fs::write(appdir.join(desktop_name), &desktop_content)?;
    fs::create_dir_all(&share_apps)?;
    fs::write(share_apps.join(desktop_name), &desktop_content)?;

    // Icons – copy all .png icons into the hicolor hierarchy.
    for icon_rel in &ctx.config.icons {
        if !icon_rel.ends_with(".png") {
            continue;
        }
        let src = ctx.icon_path(icon_rel);
        if !src.exists() {
            continue;
        }
        // Determine size from filename (e.g. "icons/128x128.png" → "128x128").
        let filename = Path::new(icon_rel)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("128x128");
        let size_str = if filename.ends_with("@2x") {
            // skip @2x variants for AppImage
            continue;
        } else {
            filename
        };

        let icon_dir = icons_dir.join(size_str).join("apps");
        fs::create_dir_all(&icon_dir)?;
        let dst = icon_dir.join("alcom.png");
        fs::copy(&src, &dst)
            .with_context(|| format!("copying icon {} → {}", src.display(), dst.display()))?;

        // Also copy the largest icon as the top-level .DirIcon for AppImage.
        if size_str == "128x128" {
            fs::copy(&src, appdir.join(".DirIcon"))?;
            // Also place an `alcom.png` at the AppDir root (appimagetool needs it).
            fs::copy(&src, appdir.join("alcom.png"))?;
        }
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
// .deb
// ---------------------------------------------------------------------------

fn create_deb(ctx: &BundleContext<'_>) -> Result<()> {
    let arch = deb_arch(ctx.target_triple);
    let pkg_name = format!("ALCOM_{}_{arch}", ctx.config.version);
    let deb_stage = ctx.bundle_dir.join("deb").join(&pkg_name);

    if deb_stage.exists() {
        fs::remove_dir_all(&deb_stage)?;
    }

    // Directory layout.
    let debian_dir = deb_stage.join("DEBIAN");
    let usr_bin = deb_stage.join("usr/bin");
    let share_apps = deb_stage.join("usr/share/applications");
    let icons_base = deb_stage.join("usr/share/icons/hicolor");

    fs::create_dir_all(&debian_dir)?;
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
    for icon_rel in &ctx.config.icons {
        if !icon_rel.ends_with(".png") {
            continue;
        }
        let src = ctx.icon_path(icon_rel);
        if !src.exists() {
            continue;
        }
        let filename = Path::new(icon_rel)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("128x128");
        if filename.ends_with("@2x") {
            continue;
        }
        let icon_dir = icons_base.join(filename).join("apps");
        fs::create_dir_all(&icon_dir)?;
        fs::copy(&src, icon_dir.join("alcom.png"))?;
    }

    // DEBIAN/control.
    let installed_size = dir_size(&deb_stage).unwrap_or(0) / 1024;
    let control = format!(
        "Package: alcom\n\
         Version: {version}\n\
         Architecture: {arch}\n\
         Installed-Size: {installed_size}\n\
         Maintainer: {publisher}\n\
         Description: {short_desc}\n\
         {long_desc}\n",
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
    fs::write(debian_dir.join("control"), control)?;

    // Build .deb.
    let deb_name = format!("{pkg_name}.deb");
    let deb_out = ctx.bundle_dir.join("deb").join(&deb_name);
    run_cmd(
        ProcessCommand::new("dpkg-deb")
            .arg("--build")
            .arg("--root-owner-group")
            .arg(&deb_stage)
            .arg(&deb_out),
        "building .deb package",
    )?;

    println!("created: {}", deb_out.display());
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
// .rpm
// ---------------------------------------------------------------------------

fn create_rpm(ctx: &BundleContext<'_>) -> Result<()> {
    let arch = rpm_arch(ctx.target_triple);
    let rpm_name = format!("ALCOM-{}-1.{arch}.rpm", ctx.config.version);
    let rpm_dir = ctx.bundle_dir.join("rpm");
    fs::create_dir_all(&rpm_dir)?;

    // rpmbuild directory structure.
    let rpmbuild = ctx.bundle_dir.join("rpmbuild");
    for sub in &["BUILD", "RPMS", "SOURCES", "SPECS", "SRPMS"] {
        fs::create_dir_all(rpmbuild.join(sub))?;
    }

    let bin_name_lower = ctx.binary_name().to_ascii_lowercase();
    let build_root = rpmbuild.join("BUILD").join("buildroot");

    let usr_bin = build_root.join("usr/bin");
    let share_apps = build_root.join("usr/share/applications");
    let icons_base = build_root.join("usr/share/icons/hicolor");

    fs::create_dir_all(&usr_bin)?;
    fs::create_dir_all(&share_apps)?;

    // Binary.
    let bin_dst = usr_bin.join(&bin_name_lower);
    fs::copy(ctx.binary_path(), &bin_dst).context("copying binary to rpm build root")?;
    make_executable(&bin_dst)?;

    // Desktop file.
    let desktop_content = render_desktop_file(ctx, &format!("/usr/bin/{bin_name_lower}"))?;
    let desktop_file = format!("{bin_name_lower}.desktop");
    fs::write(share_apps.join(&desktop_file), &desktop_content)?;

    // Icons.
    for icon_rel in &ctx.config.icons {
        if !icon_rel.ends_with(".png") {
            continue;
        }
        let src = ctx.icon_path(icon_rel);
        if !src.exists() {
            continue;
        }
        let filename = Path::new(icon_rel)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("128x128");
        if filename.ends_with("@2x") {
            continue;
        }
        let icon_dir = icons_base.join(filename).join("apps");
        fs::create_dir_all(&icon_dir)?;
        fs::copy(&src, icon_dir.join("alcom.png"))?;
    }

    // Collect file list for the RPM spec.
    let file_list = collect_files(&build_root)?;

    // Spec file.
    let spec_content = render_rpm_spec(ctx, arch, &build_root, &file_list)?;
    let spec_path = rpmbuild.join("SPECS").join("alcom.spec");
    fs::write(&spec_path, spec_content)?;

    // Run rpmbuild.
    run_cmd(
        ProcessCommand::new("rpmbuild")
            .arg("-bb")
            .arg(&spec_path)
            .arg("--define")
            .arg(format!("_topdir {}", rpmbuild.display())),
        "building RPM package",
    )?;

    // Move the output RPM to bundle/rpm/.
    let rpm_built = rpmbuild
        .join("RPMS")
        .join(arch)
        .join(&rpm_name);
    let rpm_out = rpm_dir.join(&rpm_name);
    if rpm_built.exists() {
        fs::rename(&rpm_built, &rpm_out)
            .with_context(|| format!("moving {} → {}", rpm_built.display(), rpm_out.display()))?;
    }

    println!("created: {}", rpm_out.display());
    Ok(())
}

/// Collect all files under `root`, relative to `root`, as `/usr/...` paths.
fn collect_files(root: &Path) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    collect_files_rec(root, root, &mut paths)?;
    Ok(paths)
}

fn collect_files_rec(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_rec(root, &path, out)?;
        } else {
            let rel = path.strip_prefix(root).unwrap();
            out.push(format!("/{}", rel.display()));
        }
    }
    Ok(())
}

fn render_rpm_spec(
    ctx: &BundleContext<'_>,
    arch: &str,
    build_root: &Path,
    files: &[String],
) -> Result<String> {
    let file_section = files.join("\n");
    let spec = format!(
        "%define _rpmfilename %%{{NAME}}-%%{{VERSION}}-%%{{RELEASE}}.%%{{ARCH}}.rpm\n\
         Name:           alcom\n\
         Version:        {version}\n\
         Release:        1\n\
         Summary:        {summary}\n\
         License:        MIT\n\
         BuildArch:      {arch}\n\
         \n\
         %description\n\
         {long_desc}\n\
         \n\
         %install\n\
         cp -a {buildroot}/. %{{buildroot}}/\n\
         \n\
         %files\n\
         {file_section}\n",
        version = ctx.config.version,
        summary = ctx.config.short_description,
        arch = arch,
        long_desc = ctx.config.long_description,
        buildroot = build_root.display(),
        file_section = file_section,
    );
    Ok(spec)
}

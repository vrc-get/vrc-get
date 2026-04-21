use super::BundleContext;
use crate::utils::tar::TarBuilderExt;
use anyhow::{Context, Result};
use std::path::Path;
use std::{fs, io};

pub fn create_install_build_root(
    ctx: &BundleContext<'_>,
    build_root_in: Option<&Path>,
) -> Result<()> {
    let build_root = ctx.bundle_dir.join("buildroot");
    let build_root = build_root_in.unwrap_or(&build_root);

    if build_root_in.is_none() {
        println!("note: you can change directory with --buildroot option")
    }

    match fs::remove_dir_all(build_root) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).context("failed to clean build root"),
    }
    fs::create_dir_all(build_root).context("failed to create build root directory")?;

    create_install_build_root_impl(ctx, &mut RealBuildRootFs(build_root))?;

    println!("created build root: {}", build_root.display());
    Ok(())
}

pub trait BuildRootFs {
    fn create_dir(&mut self, path: &str) -> Result<()>;
    fn create_file(&mut self, mode: u32, path: &str, data: &mut dyn io::Read) -> Result<()>;
}

pub fn create_install_build_root_impl(
    ctx: &BundleContext<'_>,
    fs: &mut dyn BuildRootFs,
) -> Result<()> {
    struct Path(String);

    impl Path {
        fn join(&self, path: &str) -> Path {
            Path(format!("{self}/{path}", self = self.0))
        }
        fn with_added_extension(&self, ext: &str) -> Path {
            Path(format!("{self}.{ext}", self = self.0))
        }
    }

    impl std::ops::Deref for Path {
        type Target = str;

        fn deref(&self) -> &Self::Target {
            self.0.as_ref()
        }
    }

    let usr = Path("usr".to_string());
    fs.create_dir(&usr)?;

    let usr_bin = usr.join("bin");
    fs.create_dir(&usr_bin)?;

    let bin_name_lower = ctx.binary_name().to_ascii_lowercase();
    let bin_path = usr_bin.join(&bin_name_lower);
    fs.create_file(
        0o755,
        &bin_path,
        &mut fs::File::open(ctx.binary_path()).context("opening executable")?,
    )?;

    let usr_share = usr.join("share");
    fs.create_dir(&usr_share)?;

    let usr_share_applications = usr_share.join("applications");
    fs.create_dir(&usr_share_applications)?;

    fs.create_file(
        0o644,
        &usr_share_applications
            .join(&bin_name_lower)
            .with_added_extension("desktop"),
        &mut render_desktop_file(ctx, &format!("/usr/bin/{bin_name_lower}"))?.as_bytes(),
    )?;

    let usr_share_icons = usr_share.join("icons");
    fs.create_dir(&usr_share_icons)?;
    let usr_share_icons_hicolor = usr_share_icons.join("hicolor");
    fs.create_dir(&usr_share_icons_hicolor)?;

    for size in LINUX_ICON_RESOLUTIONS {
        let size_dir = usr_share_icons_hicolor.join(size);
        let size_apps = size_dir.join("apps");

        fs.create_dir(&size_dir)?;
        fs.create_dir(&size_apps)?;

        fs.create_file(
            0o644,
            &size_apps.join(LINUX_ICON_NAME).with_added_extension("png"),
            &mut fs::File::open(ctx.icon_path(size)).context("opening icon file")?,
        )?;
    }
    Ok(())
}

struct RealBuildRootFs<'a>(&'a Path);

impl<'a> BuildRootFs for RealBuildRootFs<'a> {
    fn create_dir(&mut self, path: &str) -> Result<()> {
        fs::create_dir_all(self.0.join(path)).with_context(|| format!("creating directory {path}"))
    }

    fn create_file(&mut self, mode: u32, relative: &str, data: &mut dyn io::Read) -> Result<()> {
        let path = &self.0.join(relative);
        std::io::copy(
            data,
            &mut fs::File::create(path).with_context(|| format!("creating {relative}"))?,
        )
        .with_context(|| format!("writing {relative}"))?;

        #[cfg(unix)]
        fs::set_permissions(
            path,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(mode),
        )
        .with_context(|| format!("setting permission of {relative}"))?;

        Ok(())
    }
}

impl<W: io::Write> BuildRootFs for tar::Builder<W> {
    fn create_dir(&mut self, path: &str) -> Result<()> {
        self.append_directory(path)
    }

    fn create_file(&mut self, mode: u32, path: &str, data: &mut dyn io::Read) -> Result<()> {
        self.append_file_data(mode, path, data)
    }
}

impl BuildRootFs for rpm::PackageBuilder {
    fn create_dir(&mut self, path: &str) -> Result<()> {
        self.with_dir_entry(rpm::FileOptions::dir(format!("/{path}")))
            .map(|_| ())
            .with_context(|| format!("creating directory {}", path))
    }

    fn create_file(&mut self, mode: u32, path: &str, data: &mut dyn io::Read) -> Result<()> {
        let mut contents = vec![];
        data.read_to_end(&mut contents)
            .with_context(|| format!("reading data for {}", path))?;

        self.with_file_contents(
            contents,
            rpm::FileOptions::new(format!("/{path}")).permissions(mode as u16),
        )
        .map(|_| ())
        .with_context(|| format!("creating file {}", path))
    }
}

/// Render the desktop file template from `alcom.desktop`.
///
/// The template uses `{{key}}` placeholders as in the tauri bundler.
pub fn render_desktop_file(ctx: &BundleContext<'_>, exec: &str) -> Result<String> {
    let template_path = ctx.gui_dir.join("bundle/alcom.desktop");
    let template = fs::read_to_string(&template_path)
        .with_context(|| format!("reading {}", template_path.display()))?;

    Ok(template.replace("{{exec}}", exec))
}

pub static LINUX_ICON_RESOLUTIONS: &[&str] = &["32x32", "64x64", "128x128"];

pub static LINUX_ICON_NAME: &str = "alcom"; // keep in sync with alcom.desktop template

use super::{create_tar_gz, run_cmd, BundleContext};
use anyhow::{Context, Result};
use plist::{Dictionary, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

/// Create all macOS bundles: `.app`, `.app.tar.gz`, and `.dmg`.
pub fn bundle(ctx: &BundleContext<'_>) -> Result<()> {
    let app_bundle = create_app_bundle(ctx)?;
    create_app_tar_gz(ctx, &app_bundle)?;
    create_dmg(ctx, &app_bundle)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// .app bundle
// ---------------------------------------------------------------------------

/// Creates `bundle/macos/ALCOM.app` and returns its path.
fn create_app_bundle(ctx: &BundleContext<'_>) -> Result<PathBuf> {
    let app_dir = ctx.bundle_dir.join("macos").join("ALCOM.app");
    let contents_dir = app_dir.join("Contents");
    let macos_dir = contents_dir.join("MacOS");
    let resources_dir = contents_dir.join("Resources");

    // Clean previous build.
    if app_dir.exists() {
        fs::remove_dir_all(&app_dir).with_context(|| format!("removing {}", app_dir.display()))?;
    }

    fs::create_dir_all(&macos_dir).with_context(|| format!("creating {}", macos_dir.display()))?;
    fs::create_dir_all(&resources_dir)
        .with_context(|| format!("creating {}", resources_dir.display()))?;

    // Copy binary.
    let src_bin = ctx.binary_path();
    let dst_bin = macos_dir.join(ctx.binary_name());
    fs::copy(&src_bin, &dst_bin).with_context(|| {
        format!(
            "copying binary {} -> {}",
            src_bin.display(),
            dst_bin.display()
        )
    })?;

    // Make binary executable (mode 755).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dst_bin, fs::Permissions::from_mode(0o755))?;
    }

    // Copy icns icon.
    {
        let icns = ctx.gui_dir.join("icons/icon.icns");
        let dst = resources_dir.join("icon.icns");
        fs::copy(&icns, &dst)
            .with_context(|| format!("copying icon {} -> {}", icns.display(), dst.display()))?;
    }

    // Generate Info.plist using the plist crate.
    let plist_value = generate_info_plist(ctx)?;
    let plist_path = contents_dir.join("Info.plist");
    plist::to_file_xml(&plist_path, &plist_value)
        .with_context(|| format!("writing {}", plist_path.display()))?;

    println!("created: {}", app_dir.display());
    Ok(app_dir)
}

/// Builds the `Info.plist` dictionary by merging tauri config with the custom `Info.plist`.
///
/// The custom `Info.plist` (if present) is used as the base, and the generated core entries
/// always override it, ensuring correctness.
fn generate_info_plist(ctx: &BundleContext<'_>) -> Result<Value> {
    let cfg = ctx.config;
    let version = &cfg.version;

    // Start with the custom Info.plist if present (provides URL types, file associations, etc.)
    let mut dict: Dictionary = {
        let custom_plist_path = ctx.gui_dir.join("Info.plist");
        if custom_plist_path.exists() {
            let val: Value = plist::from_file(&custom_plist_path)
                .with_context(|| format!("reading {}", custom_plist_path.display()))?;
            match val {
                Value::Dictionary(d) => d,
                _ => Dictionary::new(),
            }
        } else {
            Dictionary::new()
        }
    };

    // Override / fill in the generated core entries (these always win).
    dict.insert(
        "CFBundleName".into(),
        Value::String(cfg.product_name.clone()),
    );
    dict.insert(
        "CFBundleExecutable".into(),
        Value::String(cfg.product_name.clone()),
    );
    dict.insert(
        "CFBundleIdentifier".into(),
        Value::String(cfg.identifier.clone()),
    );
    dict.insert("CFBundleVersion".into(), Value::String(version.clone()));
    dict.insert(
        "CFBundleShortVersionString".into(),
        Value::String(version.clone()),
    );
    dict.insert("CFBundleIconFile".into(), Value::String("icon.icns".into()));
    dict.insert("CFBundlePackageType".into(), Value::String("APPL".into()));
    dict.insert("NSHighResolutionCapable".into(), Value::Boolean(true));
    dict.insert(
        "NSRequiresAquaSystemAppearance".into(),
        Value::Boolean(false),
    );
    dict.insert(
        "LSMinimumSystemVersion".into(),
        Value::String("10.13.0".into()),
    );

    if !cfg.copyright.is_empty() {
        dict.insert(
            "NSHumanReadableCopyright".into(),
            Value::String(cfg.copyright.clone()),
        );
    }

    Ok(Value::Dictionary(dict))
}

// ---------------------------------------------------------------------------
// .app.tar.gz
// ---------------------------------------------------------------------------

fn create_app_tar_gz(ctx: &BundleContext<'_>, app_bundle: &Path) -> Result<()> {
    let out = ctx.bundle_dir.join("macos").join("ALCOM.app.tar.gz");
    // Archive the .app directory with the name "ALCOM.app" inside the tarball.
    create_tar_gz(app_bundle, "ALCOM.app", &out)
}

// ---------------------------------------------------------------------------
// DMG
// ---------------------------------------------------------------------------

/// Creates `bundle/dmg/ALCOM_<version>_<arch>.dmg` using `hdiutil`.
fn create_dmg(ctx: &BundleContext<'_>, app_bundle: &Path) -> Result<()> {
    let arch = dmg_arch(ctx.target_triple);
    let dmg_name = format!("ALCOM_{}_{arch}.dmg", ctx.config.version);
    let dmg_dir = ctx.bundle_dir.join("dmg");
    fs::create_dir_all(&dmg_dir).with_context(|| format!("creating {}", dmg_dir.display()))?;
    let dmg_path = dmg_dir.join(&dmg_name);

    if dmg_path.exists() {
        fs::remove_file(&dmg_path)?;
    }

    // Stage directory: app + /Applications symlink.
    let staging = ctx.bundle_dir.join("dmg-staging");
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;

    // Copy the .app bundle into the staging area.
    run_cmd(
        ProcessCommand::new("cp")
            .arg("-R")
            .arg(app_bundle)
            .arg(&staging),
        "copying .app to DMG staging",
    )?;

    // Create a symlink to /Applications.
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink("/Applications", staging.join("Applications"))
            .context("creating /Applications symlink")?;
    }

    // Create the DMG with hdiutil.
    let volume_name = &ctx.config.product_name;
    run_cmd(
        ProcessCommand::new("hdiutil")
            .arg("create")
            .arg(&dmg_path)
            .arg("-volname")
            .arg(volume_name)
            .arg("-fs")
            .arg("HFS+")
            .arg("-srcfolder")
            .arg(&staging)
            .arg("-ov")
            .arg("-format")
            .arg("UDZO"),
        "creating DMG with hdiutil",
    )?;

    println!("created: {}", dmg_path.display());
    Ok(())
}

fn dmg_arch(triple: &str) -> &str {
    if triple.contains("universal") {
        "universal"
    } else if triple.contains("aarch64") {
        "aarch64"
    } else {
        "x86_64"
    }
}

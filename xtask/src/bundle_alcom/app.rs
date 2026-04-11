use super::{BundleContext, create_tar_gz};
use crate::utils::make_executable;
use anyhow::{Context, Result};
use std::fs;

/// Creates `bundle/macos/ALCOM.app`.
pub fn create_app_bundle(ctx: &BundleContext<'_>) -> Result<()> {
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
    make_executable(&dst_bin)?;

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
    Ok(())
}

/// Builds the `Info.plist` dictionary by merging tauri config with the custom `Info.plist`.
///
/// The custom `Info.plist` (if present) is used as the base, and the generated core entries
/// always override it, ensuring correctness.
fn generate_info_plist(ctx: &BundleContext<'_>) -> Result<plist::Value> {
    use plist::Value;

    let cfg = ctx.config;
    let version = &cfg.version;

    // Start with the custom Info.plist if present (provides URL types, file associations, etc.)
    let mut dict: plist::Dictionary = {
        let custom_plist_path = ctx.gui_dir.join("Info.plist");
        if custom_plist_path.exists() {
            let val: Value = plist::from_file(&custom_plist_path)
                .with_context(|| format!("reading {}", custom_plist_path.display()))?;
            match val {
                Value::Dictionary(d) => d,
                _ => plist::Dictionary::new(),
            }
        } else {
            plist::Dictionary::new()
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

pub fn create_app_tar_gz(ctx: &BundleContext<'_>) -> Result<()> {
    let app_bundle = ctx.bundle_dir.join("macos").join("ALCOM.app");

    let out = ctx.bundle_dir.join("macos").join("ALCOM.app.tar.gz");
    // Archive the .app directory with the name "ALCOM.app" inside the tarball.
    create_tar_gz(&app_bundle, "ALCOM.app", &out)
}

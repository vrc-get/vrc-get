use super::BundleContext;
use crate::utils::command::CommandExt;
use crate::utils::ds_store::{DsStore, EntryValue};
use crate::utils::target_arch;
use anyhow::Context;
use std::fs;
use std::process::Command as ProcessCommand;

pub fn create_dmg(ctx: &BundleContext<'_>) -> anyhow::Result<()> {
    let app_bundle = ctx.bundle_dir.join("macos").join("ALCOM.app");
    let arch = target_arch(ctx.target_truple);
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

    // Copy the .app bundle into the staging area using fs_extra.
    {
        let copy_options = fs_extra::dir::CopyOptions::new().copy_inside(false);
        fs_extra::dir::copy(&app_bundle, &staging, &copy_options).with_context(|| {
            format!(
                "copying .app bundle {} -> {}",
                app_bundle.display(),
                staging.display()
            )
        })?;
    }

    // Create a symlink to /Applications.
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink("/Applications", staging.join("Applications"))
            .context("creating /Applications symlink")?;
    }

    // Write a .DS_Store so Finder displays the DMG with correct icon positions.
    //
    // Layout: ALCOM.app on the left, Applications symlink on the right.
    let ds_store_path = staging.join(".DS_Store");
    dmg_ds_store("ALCOM.app")
        .write_to(&ds_store_path)
        .context("writing .DS_Store for DMG staging")?;

    // Create the DMG with hdiutil.
    let volume_name = &ctx.config.product_name;

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
        .arg("UDZO")
        .run_checked("creating DMG with hdiutil")?;

    println!("created: {}", dmg_path.display());
    Ok(())
}

/// Build a `.DS_Store` for the macOS disk image.
pub fn dmg_ds_store(app_name: &str) -> DsStore {
    let mut store = DsStore::new();

    // bwsp — Finder window bounds (stored as a binary plist).
    let bwsp = binary_plist(plist::Dictionary::from_iter([
        (
            "ContainerShowSidebar".to_string(),
            plist::Value::Boolean(false),
        ),
        (
            "PreviewPaneVisibility".to_string(),
            plist::Value::Boolean(false),
        ),
        ("ShowStatusBar".to_string(), plist::Value::Boolean(false)),
        ("SidebarWidth".to_string(), plist::Value::Integer(0.into())),
        (
            "WindowBounds".to_string(),
            plist::Value::String("{{10, 620}, {660, 400}}".into()),
        ),
    ]));
    store.insert(".", b"bwsp", EntryValue::Blob(bwsp));

    // icvp — icon view properties (stored as a binary plist).
    let icvp = binary_plist(plist::Dictionary::from_iter([
        (
            "viewOptionsVersion".to_string(),
            plist::Value::Integer(1.into()),
        ),
        (
            "arrangeBy".to_string(),
            plist::Value::String("none".to_string()),
        ),
        // #58BB81 is the main color of the ALCOM icon
        ("backgroundColorRed".to_string(), 0.345.into()),
        ("backgroundColorGreen".to_string(), 0.733.into()),
        ("backgroundColorBlue".to_string(), 0.506.into()),
        (
            "backgroundType".to_string(),
            // 0 for none,
            // 1 for solid color
            // 2 for image (BKGD record or backgroundImageAlias key)
            plist::Value::Integer(1.into()),
        ),
        ("gridOffsetX".to_string(), plist::Value::Real(0.0)),
        ("gridOffsetY".to_string(), plist::Value::Real(0.0)),
        ("gridSpacing".to_string(), plist::Value::Real(100.0)),
        ("iconSize".to_string(), plist::Value::Real(128.0)),
        ("labelOnBottom".to_string(), plist::Value::Boolean(true)),
        ("scrollPositionX".to_string(), plist::Value::Real(0.0)),
        ("scrollPositionY".to_string(), plist::Value::Real(0.0)),
        ("showIconPreview".to_string(), plist::Value::Boolean(true)),
        ("showItemInfo".to_string(), plist::Value::Boolean(false)),
        ("textSize".to_string(), plist::Value::Real(16.0)),
    ]));
    store.insert(".", b"icvp", EntryValue::Blob(icvp));

    // vSrn — view sort version.
    store.insert(".", b"vSrn", EntryValue::Long(1));

    // Icon positions.
    store.set_icon_location(app_name, 180, 170);
    store.set_icon_location("Applications", 480, 170);

    store
}

fn binary_plist(v: impl Into<plist::Value>) -> Vec<u8> {
    let mut buf = Vec::new();
    plist::to_writer_binary(&mut buf, &v.into()).expect("plist serialisation");
    buf
}

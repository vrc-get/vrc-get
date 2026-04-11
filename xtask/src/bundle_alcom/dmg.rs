use super::BundleContext;
use crate::utils::command::CommandExt;
use crate::utils::ds_store::dmg_ds_store;
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
    dmg_ds_store(
        "ALCOM.app",
        170, // app icon X
        220, // app icon Y
        430, // Applications icon X
        220, // Applications icon Y
    )
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


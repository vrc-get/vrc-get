use super::BundleContext;
use crate::utils::command::CommandExt;
use crate::utils::ds_store::{DsStore, EntryValue};
use crate::utils::target_arch;
use anyhow::Context;
use std::fs;
use std::process::Command as ProcessCommand;

static BACKGROUND_FILE_NAME: &str = ".background.tiff";

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

    fs::copy(
        ctx.gui_dir.join("mac-background.tiff"),
        staging.join(BACKGROUND_FILE_NAME),
    )
    .with_context(|| format!("Creating {BACKGROUND_FILE_NAME}"))?;

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
            plist::Value::Integer(2.into()),
        ),
        (
            "backgroundImageAlias".to_string(),
            plist::Value::Data(alias::create_alias_for_relative_path(
                "ALCOM",
                BACKGROUND_FILE_NAME,
            )),
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
    store.set_icon_location(BACKGROUND_FILE_NAME, 128, 128);

    store
}

fn binary_plist(v: impl Into<plist::Value>) -> Vec<u8> {
    let mut buf = Vec::new();
    plist::to_writer_binary(&mut buf, &v.into()).expect("plist serialisation");
    buf
}

mod alias {
    pub fn create_alias_for_relative_path(volume_name: &str, relative_path: &str) -> Vec<u8> {
        let file_name = relative_path;

        let mount_path = format!("/Volumes/{volume_name}");
        let in_volume_path = format!("/{relative_path}");
        let full_path = if mount_path == "/" {
            in_volume_path.to_string()
        } else {
            format!("{mount_path}{in_volume_path}")
        };
        let folder_name = full_path
            .rsplit_once('/')
            .map(|(dir_path, _filename)| {
                let (_parent_path, dir_name) = dir_path.rsplit_once('/').unwrap_or(("", dir_path));
                dir_name
            })
            .unwrap_or("");

        let mut buf = Vec::new();

        // 80 bytes header
        // User Type: 'alis'
        //buf.extend_from_slice(b"alis");
        buf.extend_from_slice(b"\0\0\0\0");

        // Size: (2 bytes) (filled later)
        buf.extend_from_slice(&0u16.to_be_bytes());

        // Version: (2 bytes) 2
        buf.extend_from_slice(&2u16.to_be_bytes());

        // Kind: (2 bytes) 0 = File, 1 = Directory
        buf.extend_from_slice(&0u16.to_be_bytes());

        // Volume Name: (28 bytes) blank
        buf.extend_from_slice(&pascal_u8_string::<28>(volume_name));

        // Volume Create Date: (4 bytes)
        buf.extend_from_slice(&0u32.to_be_bytes());

        // FS Type: (2 bytes) 'H+' (HFS+)
        buf.extend_from_slice(b"H+");

        // Disk Type: (2 bytes) 0
        buf.extend_from_slice(&0u16.to_be_bytes());

        // Parent Directory ID: (4 bytes) 0
        buf.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes());

        // File Name: (64 bytes Pascal String)
        // single byte represents length followed by name
        buf.extend_from_slice(&pascal_u8_string::<64>(file_name));

        // File ID: (4 bytes) 0
        buf.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes());

        // Create Date: (4 bytes) 0
        buf.extend_from_slice(&0u32.to_be_bytes());

        // Creator / Type: (8 bytes) 0
        buf.extend_from_slice(&[0u8; 8]);

        // Levels: (4 bytes) -1, -1 (Default)
        buf.extend_from_slice(&(-1i16).to_be_bytes());
        buf.extend_from_slice(&(-1i16).to_be_bytes());

        // Volume Attributes: (4 bytes) 0
        buf.extend_from_slice(&0x00000000u32.to_be_bytes());

        // FileSystem Id
        buf.extend_from_slice(b"\0\0");

        // unknown padding
        buf.extend_from_slice(&[0u8; 10]);

        // 2. tagged extended data
        // ---------------------------------------------------------
        // Tag 0: carbon folder name
        append_tag(&mut buf, 0, folder_name.as_bytes());

        // Tag 2: Target's carbon path (':' is the path separator)
        append_tag(
            &mut buf,
            2,
            format!("/{}", full_path.replace("/", ":")).as_bytes(),
        );

        // Tag 14: Unicode FileName
        append_tag(&mut buf, 14, &pascal_variadic_u16(file_name));

        // Tag 15: Unicode Volume Name
        append_tag(&mut buf, 15, &pascal_variadic_u16(volume_name));

        // Tag 18: Posix path
        append_tag(&mut buf, 18, in_volume_path.as_bytes());

        // Tag 19: Posix path to mountpoint
        append_tag(&mut buf, 19, mount_path.as_bytes());

        // End Tag: -1 (0xFFFF)
        buf.extend_from_slice(&(-1i16).to_be_bytes());
        buf.extend_from_slice(&0u16.to_be_bytes());

        // 3. update total length
        // ---------------------------------------------------------
        let total_len = buf.len() as u16;
        buf[4..][..2].copy_from_slice(&total_len.to_be_bytes());

        buf
    }

    fn pascal_u8_string<const N: usize>(name: &str) -> [u8; N] {
        let mut buf = [0u8; N];
        buf[0] = name.len() as u8;
        buf[1..][..name.len()].copy_from_slice(name.as_bytes());
        buf
    }

    fn pascal_variadic_u16(string: &str) -> Vec<u8> {
        let mut buf = vec![];
        buf.extend_from_slice(&(string.encode_utf16().count() as u16).to_be_bytes());
        (string.encode_utf16())
            .flat_map(u16::to_be_bytes)
            .for_each(|x| buf.push(x));
        buf
    }

    fn append_tag(buf: &mut Vec<u8>, tag: i16, data: &[u8]) {
        buf.extend_from_slice(&tag.to_be_bytes());
        buf.extend_from_slice(&(data.len() as u16).to_be_bytes());
        buf.extend_from_slice(data);
        if !data.len().is_multiple_of(2) {
            buf.push(0); // padding for 2 byte alignment
        }
    }
}

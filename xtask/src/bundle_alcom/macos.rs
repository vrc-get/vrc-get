use super::{BundleContext, create_tar_gz, run_cmd};
use anyhow::{Context, Result};
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
        fs::remove_dir_all(&app_dir)
            .with_context(|| format!("removing {}", app_dir.display()))?;
    }

    fs::create_dir_all(&macos_dir)
        .with_context(|| format!("creating {}", macos_dir.display()))?;
    fs::create_dir_all(&resources_dir)
        .with_context(|| format!("creating {}", resources_dir.display()))?;

    // Copy binary.
    let src_bin = ctx.binary_path();
    let dst_bin = macos_dir.join(ctx.binary_name());
    fs::copy(&src_bin, &dst_bin)
        .with_context(|| format!("copying binary {} → {}", src_bin.display(), dst_bin.display()))?;

    // Make binary executable (mode 755).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dst_bin, fs::Permissions::from_mode(0o755))?;
    }

    // Copy icns icon.
    if let Some(icns) = ctx.find_icon(".icns") {
        let dst = resources_dir.join("icon.icns");
        fs::copy(&icns, &dst)
            .with_context(|| format!("copying icon {} → {}", icns.display(), dst.display()))?;
    }

    // Generate Info.plist.
    let info_plist = generate_info_plist(ctx)?;
    let plist_path = contents_dir.join("Info.plist");
    fs::write(&plist_path, info_plist)
        .with_context(|| format!("writing {}", plist_path.display()))?;

    println!("created: {}", app_dir.display());
    Ok(app_dir)
}

/// Builds the `Info.plist` XML by merging tauri config with the custom `Info.plist`.
fn generate_info_plist(ctx: &BundleContext<'_>) -> Result<String> {
    let cfg = ctx.config;
    let version = &cfg.version;

    // Core plist entries generated from bundle config.
    let core_entries: Vec<(&str, String)> = vec![
        ("CFBundleName", plist_string(&cfg.product_name)),
        ("CFBundleExecutable", plist_string(&cfg.product_name)),
        ("CFBundleIdentifier", plist_string(&cfg.identifier)),
        ("CFBundleVersion", plist_string(version)),
        ("CFBundleShortVersionString", plist_string(version)),
        ("CFBundleIconFile", plist_string("icon.icns")),
        ("CFBundlePackageType", plist_string("APPL")),
        ("NSHighResolutionCapable", "<true/>".to_string()),
        ("NSRequiresAquaSystemAppearance", "<false/>".to_string()),
        ("LSMinimumSystemVersion", plist_string("10.13.0")),
    ];

    // Merge entries from the custom Info.plist if present.
    let custom_plist_path = ctx.gui_dir.join("Info.plist");
    let custom_entries: Vec<(String, String)> = if custom_plist_path.exists() {
        parse_plist_dict_entries(&fs::read_to_string(&custom_plist_path)?)?
    } else {
        vec![]
    };

    // Serialize: core entries first, then custom entries that don't duplicate core keys.
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n",
    );

    for (key, value) in &core_entries {
        out.push_str(&format!("    <key>{key}</key>\n    {value}\n"));
    }

    if !cfg.copyright.is_empty() {
        out.push_str(&format!(
            "    <key>NSHumanReadableCopyright</key>\n    {}\n",
            plist_string(&cfg.copyright)
        ));
    }

    // Append custom plist entries that are not already set by the core list.
    let core_keys: Vec<&str> = core_entries.iter().map(|(k, _)| *k).collect();
    for (key, value) in &custom_entries {
        if !core_keys.contains(&key.as_str()) {
            out.push_str(&format!("    <key>{key}</key>\n    {value}\n"));
        }
    }

    out.push_str("</dict>\n</plist>\n");
    Ok(out)
}

fn plist_string(s: &str) -> String {
    format!("<string>{}</string>", xml_escape(s))
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Parse the top-level `<dict>` of a plist XML file and return key/value pairs.
/// Values are returned as raw XML strings (e.g. `<string>foo</string>`, `<true/>`).
fn parse_plist_dict_entries(src: &str) -> Result<Vec<(String, String)>> {
    let mut result: Vec<(String, String)> = Vec::new();
    let mut rest = src;

    // Skip until the first <dict>
    if let Some(pos) = rest.find("<dict>") {
        rest = &rest[pos + 6..];
    }

    loop {
        // Skip whitespace
        rest = rest.trim_start();

        if rest.starts_with("</dict>") || rest.is_empty() {
            break;
        }

        if rest.starts_with("<key>") {
            rest = &rest[5..]; // skip <key>
            let end = rest.find("</key>").context("missing </key>")?;
            let key = rest[..end].to_owned();
            rest = &rest[end + 6..]; // skip </key>
            rest = rest.trim_start();

            // Read the value tag(s)
            let (value, after) = read_plist_value(rest)?;
            result.push((key, value));
            rest = after;
        } else {
            // Skip unexpected content
            if let Some(next) = rest.find('<') {
                rest = &rest[next..];
                let end = rest.find('>').map(|p| p + 1).unwrap_or(rest.len());
                rest = &rest[end..];
            } else {
                break;
            }
        }
    }

    Ok(result)
}

/// Read one plist value starting at `src` (which should start with the opening tag).
/// Returns `(raw_xml_string, remaining_src)`.
fn read_plist_value<'a>(src: &'a str) -> Result<(String, &'a str)> {
    let src = src.trim_start();

    if src.starts_with("<true/>") {
        return Ok(("<true/>".to_owned(), &src[7..]));
    }
    if src.starts_with("<false/>") {
        return Ok(("<false/>".to_owned(), &src[8..]));
    }

    // Find the opening tag
    let end_of_open = src.find('>').context("missing > in plist value tag")?;
    let open_tag = &src[..end_of_open + 1];

    // Determine the tag name for the closing tag
    let tag_name_end = src[1..].find(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .map(|p| p + 1)
        .unwrap_or(end_of_open);
    let tag_name = &src[1..tag_name_end];

    if open_tag.ends_with("/>") {
        // Self-closing
        return Ok((open_tag.to_owned(), &src[open_tag.len()..]));
    }

    let close_tag = format!("</{tag_name}>");

    // Find matching closing tag (handles nesting for array/dict)
    let after_open = &src[end_of_open + 1..];
    let (inner, rest) = find_matching_close(after_open, tag_name, &close_tag)?;
    let raw = format!("{open_tag}{inner}{close_tag}");
    Ok((raw, rest))
}

/// Find the matching closing tag, handling nested same-named tags.
fn find_matching_close<'a>(
    src: &'a str,
    tag_name: &str,
    close_tag: &str,
) -> Result<(&'a str, &'a str)> {
    let open_tag_str = format!("<{tag_name}");
    let mut depth = 1usize;
    let mut i = 0;

    while i < src.len() {
        if src[i..].starts_with(close_tag) {
            depth -= 1;
            if depth == 0 {
                return Ok((&src[..i], &src[i + close_tag.len()..]));
            }
            i += close_tag.len();
        } else if src[i..].starts_with(&open_tag_str) {
            // Check it's a full tag name (not a prefix match)
            let after = &src[i + open_tag_str.len()..];
            if after.starts_with(|c: char| c.is_whitespace() || c == '>' || c == '/') {
                depth += 1;
            }
            i += open_tag_str.len();
        } else {
            i += 1;
        }
    }

    Err(anyhow::anyhow!("missing closing tag {close_tag}"))
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
    fs::create_dir_all(&dmg_dir)
        .with_context(|| format!("creating {}", dmg_dir.display()))?;
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

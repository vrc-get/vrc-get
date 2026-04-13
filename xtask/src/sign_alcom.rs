use crate::utils::build_dir;
use crate::utils::command::CommandExt;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

/// Signs the ALCOM.app macOS application bundle with `codesign`.
///
/// This command is intended to run **after** `bundle-alcom --bundles app` and
/// **before** `bundle-alcom --bundles dmg,app-updater`, so that the DMG and
/// updater payload contain the signed application.
///
/// Certificate and identity are read from the environment variables listed below
/// (matching the convention used by tauri's build action):
///
/// - `APPLE_CERTIFICATE`          — base64-encoded `.p12` signing certificate
/// - `APPLE_CERTIFICATE_PASSWORD` — password for the `.p12` file
/// - `APPLE_SIGNING_IDENTITY`     — signing identity string,
///   e.g. `"Developer ID Application: Your Name (TEAMID)"`
///
/// If `APPLE_CERTIFICATE` is set the certificate is imported into a temporary
/// keychain that is destroyed after signing.  If the certificate is already
/// available in the system / login keychain you can omit `APPLE_CERTIFICATE`
/// and only set `APPLE_SIGNING_IDENTITY`.
///
/// After signing the app is optionally notarized and stapled when
/// `APPLE_ID`, `APPLE_PASSWORD`, and `APPLE_TEAM_ID` are all set (or when
/// `--notarize` is passed explicitly).
#[derive(clap::Parser)]
pub(super) struct Command {
    /// Target triple (e.g. `universal-apple-darwin`).
    ///
    /// Defaults to the host triple.
    #[arg(long)]
    target: Option<String>,

    /// Build profile (default: `release`).
    #[arg(long, default_value = "release")]
    profile: String,

    /// Apple signing identity (e.g. `"Developer ID Application: …"`).
    ///
    /// Can also be set via the `APPLE_SIGNING_IDENTITY` environment variable.
    #[arg(long, env = "APPLE_SIGNING_IDENTITY")]
    identity: String,

    /// Path to a custom entitlements `.plist` file.
    ///
    /// If not provided, a default set of entitlements is generated automatically.
    #[arg(long)]
    entitlements: Option<PathBuf>,

    /// Force notarization even when `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID`
    /// are not set (will fail if they are absent).
    #[arg(long)]
    notarize: bool,
}

impl crate::Command for Command {
    fn run(self) -> Result<i32> {
        let build_dir = build_dir(self.target.as_deref(), &self.profile);

        let app_path = build_dir.join("bundle/macos/ALCOM.app");
        if !app_path.exists() {
            anyhow::bail!(
                "ALCOM.app not found at {}; run `bundle-alcom --bundles app` first",
                app_path.display()
            );
        }

        // Import the certificate if provided via env var.
        let _keychain_guard = import_certificate()?;

        // Write entitlements to a temp file.
        let tmp_dir = tempdir()?;
        let entitlements_path = match &self.entitlements {
            Some(p) => p.clone(),
            None => {
                let p = tmp_dir.join("entitlements.plist");
                write_default_entitlements(&p)?;
                p
            }
        };

        // Sign the app bundle.
        sign_app(&app_path, &self.identity, &entitlements_path)?;

        // Notarize if requested or if all required env vars are present.
        let should_notarize = self.notarize
            || (std::env::var_os("APPLE_ID").is_some()
                && std::env::var_os("APPLE_PASSWORD").is_some()
                && std::env::var_os("APPLE_TEAM_ID").is_some());

        if should_notarize {
            notarize_and_staple(&app_path)?;
        }

        Ok(0)
    }
}

// ---------------------------------------------------------------------------
// Certificate import
// ---------------------------------------------------------------------------

/// RAII guard that deletes the temporary keychain when dropped.
struct KeychainGuard {
    name: String,
}

impl Drop for KeychainGuard {
    fn drop(&mut self) {
        let _ = ProcessCommand::new("security")
            .args(["delete-keychain", &self.name])
            .status();
    }
}

/// Imports the signing certificate from the `APPLE_CERTIFICATE` env var (base64-encoded
/// `.p12`) into a temporary keychain.  Returns `None` if the env var is not set.
fn import_certificate() -> Result<Option<KeychainGuard>> {
    use base64::Engine;

    let cert_b64 = match std::env::var("APPLE_CERTIFICATE") {
        Ok(v) if !v.is_empty() => v,
        _ => return Ok(None),
    };
    let password = std::env::var("APPLE_CERTIFICATE_PASSWORD").unwrap_or_default();

    // Decode the base64-encoded certificate.
    let cert_bytes = base64::engine::general_purpose::STANDARD
        .decode(
            cert_b64
                .as_bytes()
                .iter()
                .copied()
                .filter(|c| !c.is_ascii_whitespace())
                .collect::<Vec<u8>>(),
        )
        .context("decoding APPLE_CERTIFICATE from base64")?;

    // Write the .p12 to a temp file.
    let tmp = tempdir()?;
    let cert_path = tmp.join("cert.p12");
    fs::write(&cert_path, &cert_bytes).context("writing cert.p12")?;

    let keychain_name = "alcom-build-signing.keychain-db";
    let keychain_password = "alcom-build-keychain-password";

    // Create a temporary keychain.
    ProcessCommand::new("security")
        .args(["create-keychain", "-p", keychain_password, keychain_name])
        .run_checked("creating temporary keychain")?;

    let guard = KeychainGuard {
        name: keychain_name.to_owned(),
    };

    ProcessCommand::new("security")
        .args(["set-keychain-settings", "-lut", "21600", keychain_name])
        .run_checked("configuring keychain settings")?;

    ProcessCommand::new("security")
        .args(["unlock-keychain", "-p", keychain_password, keychain_name])
        .run_checked("unlocking keychain")?;

    // Import the certificate.
    ProcessCommand::new("security")
        .arg("import")
        .arg(&cert_path)
        .args([
            "-P",
            &password,
            "-A",
            "-t",
            "cert",
            "-f",
            "pkcs12",
            "-k",
            keychain_name,
        ])
        .run_checked("importing signing certificate")?;

    // Add the keychain to the search list.
    ProcessCommand::new("security")
        .args([
            "list-keychain",
            "-d",
            "user",
            "-s",
            keychain_name,
            "login.keychain-db",
        ])
        .run_checked("adding keychain to search list")?;

    // Allow codesign (and other Apple tools) to access the key without a UI prompt.
    ProcessCommand::new("security")
        .args([
            "set-key-partition-list",
            "-S",
            "apple-tool:,apple:,codesign:",
            "-s",
            "-k",
            keychain_password,
            keychain_name,
        ])
        .run_checked("setting key partition list")?;

    println!("imported signing certificate into temporary keychain");
    Ok(Some(guard))
}

// ---------------------------------------------------------------------------
// Entitlements
// ---------------------------------------------------------------------------

/// Write a default entitlements plist suitable for a hardened-runtime macOS app.
fn write_default_entitlements(path: &Path) -> Result<()> {
    let mut dict = plist::Dictionary::new();
    // Hardened Runtime entitlements — matches what tauri generates by default.
    dict.insert(
        "com.apple.security.cs.allow-jit".into(),
        plist::Value::Boolean(false),
    );
    dict.insert(
        "com.apple.security.cs.allow-unsigned-executable-memory".into(),
        plist::Value::Boolean(false),
    );
    dict.insert(
        "com.apple.security.cs.disable-library-validation".into(),
        plist::Value::Boolean(false),
    );
    dict.insert(
        "com.apple.security.cs.allow-dyld-environment-variables".into(),
        plist::Value::Boolean(false),
    );
    dict.insert(
        "com.apple.security.network.client".into(),
        plist::Value::Boolean(true),
    );

    plist::to_file_xml(path, &plist::Value::Dictionary(dict))
        .with_context(|| format!("writing entitlements to {}", path.display()))
}

// ---------------------------------------------------------------------------
// Codesign
// ---------------------------------------------------------------------------

/// Run `codesign` to sign the `.app` bundle.
fn sign_app(app_path: &Path, identity: &str, entitlements: &Path) -> Result<()> {
    ProcessCommand::new("codesign")
        .arg("--deep")
        .arg("--force")
        .arg("--options")
        .arg("runtime")
        .arg("--sign")
        .arg(identity)
        .arg("--entitlements")
        .arg(entitlements)
        .arg(app_path)
        .run_checked("codesigning ALCOM.app")?;

    println!("signed: {}", app_path.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// Notarization
// ---------------------------------------------------------------------------

/// Notarize the signed `.app` and staple the notarization ticket.
///
/// Reads `APPLE_ID`, `APPLE_PASSWORD`, and `APPLE_TEAM_ID` from the environment.
fn notarize_and_staple(app_path: &Path) -> Result<()> {
    let apple_id = std::env::var("APPLE_ID").context("APPLE_ID env var not set")?;
    let apple_password =
        std::env::var("APPLE_PASSWORD").context("APPLE_PASSWORD env var not set")?;
    let apple_team_id = std::env::var("APPLE_TEAM_ID").context("APPLE_TEAM_ID env var not set")?;

    // Create a zip of the .app for submission (notarytool accepts .zip, .dmg, or .pkg).
    let tmp = tempdir()?;
    let zip_path = tmp.join("ALCOM.app.zip");

    ProcessCommand::new("ditto")
        .arg("-c")
        .arg("-k")
        .arg("--keepParent")
        .arg(app_path)
        .arg(&zip_path)
        .run_checked("zipping ALCOM.app for notarization")?;

    // Submit for notarization and wait.
    ProcessCommand::new("xcrun")
        .arg("notarytool")
        .arg("submit")
        .arg(&zip_path)
        .arg("--apple-id")
        .arg(&apple_id)
        .arg("--password")
        .arg(&apple_password)
        .arg("--team-id")
        .arg(&apple_team_id)
        .arg("--wait")
        .run_checked("notarizing ALCOM.app")?;

    // Staple the notarization ticket.
    ProcessCommand::new("xcrun")
        .arg("stapler")
        .arg("staple")
        .arg(app_path)
        .run_checked("stapling notarization ticket to ALCOM.app")?;

    println!("notarized and stapled: {}", app_path.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a temporary directory and return its path.
/// The caller is responsible for keeping the return value alive for the
/// duration of its use (it is *not* a RAII guard — we use a plain PathBuf
/// so we can pass it around without lifetimes; the directory is cleaned up
/// by the OS on process exit).
///
/// For the keychain guard we rely on Drop; for other uses the temp files are
/// short-lived and will be cleaned up by the OS.
fn tempdir() -> Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("xtask-sign-{}", std::process::id()));
    fs::create_dir_all(&dir).with_context(|| format!("creating temp dir {}", dir.display()))?;
    Ok(dir)
}

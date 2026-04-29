//! Updater module
//!
//! This module reimplements the auto-update logic previously provided by
//! tauri-plugin-updater in order to fix several issues:
//! - macOS: Extract to a directory on the same filesystem as the app bundle to
//!   avoid cross-device rename errors when the app is installed on a non-default
//!   volume.
//! - Windows: Support custom installer types beyond NSIS and run them in a way
//!   that correctly triggers UAC elevation.
//! - Check-for-update-only mode: Because checking and installing are separate
//!   functions, callers can check without installing (useful when ALCOM is
//!   managed by a package manager).
//!
//! This is based heavily on the tauri-plugin-updater source code.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use base64::Engine as _;
use futures::StreamExt as _;
use minisign_verify::{PublicKey, Signature};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{StatusCode, header};
use semver::Version;
use serde::{Deserialize, Deserializer, de::Error as DeError};
use tauri::{AppHandle, Env, Manager as _, Runtime};
use url::Url;

// ---------------------------------------------------------------------------
// constants
// ---------------------------------------------------------------------------

static PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDkyMjAzMkU2Q0ZGQjQ0MjYKUldRbVJQdlA1aklna2d2NnRoM3ZsT3lzWEQ3MC9zTGpaWVR4NGdQOXR0UGJaOHBlY2xCcFY5bHcK";

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum Error {
    FailedToDetermineExtractPath,
    BinaryNotFoundInArchive,
    TempDirNotOnSameMountPoint,
    Network(String),
    Signature(String),
    SignatureUtf8(String),
    InvalidBase64(base64::DecodeError),
    Json(serde_json::Error),
    Io(std::io::Error),
    Reqwest(reqwest::Error),
    Url(url::ParseError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedToDetermineExtractPath => {
                write!(f, "failed to determine extract path")
            }
            Self::BinaryNotFoundInArchive => write!(f, "binary not found in archive"),
            Self::TempDirNotOnSameMountPoint => {
                write!(f, "temp dir not on same mount point")
            }
            Self::Network(s) => write!(f, "network error: {s}"),
            Self::Signature(s) => write!(f, "signature error: {s}"),
            Self::SignatureUtf8(s) => write!(f, "signature utf8 error: {s}"),
            Self::InvalidBase64(e) => write!(f, "base64 decode error: {e}"),
            Self::Json(e) => write!(f, "json error: {e}"),
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Reqwest(e) => write!(f, "reqwest error: {e}"),
            Self::Url(e) => write!(f, "url parse error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<base64::DecodeError> for Error {
    fn from(e: base64::DecodeError) -> Self {
        Self::InvalidBase64(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Reqwest(e)
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Self::Url(e)
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

// ---------------------------------------------------------------------------
// Signature verification
// ---------------------------------------------------------------------------

fn verify_signature(data: &[u8], release_signature: &str, pub_key: &str) -> Result<bool> {
    let pub_key_decoded = base64_to_string(pub_key)?;
    let public_key =
        PublicKey::decode(&pub_key_decoded).map_err(|e| Error::Signature(e.to_string()))?;
    let sig_decoded = base64_to_string(release_signature)?;
    let signature = Signature::decode(&sig_decoded).map_err(|e| Error::Signature(e.to_string()))?;
    public_key
        .verify(data, &signature, true)
        .map_err(|e| Error::Signature(e.to_string()))?;
    Ok(true)
}

fn base64_to_string(base64_string: &str) -> Result<String> {
    let decoded = base64::engine::general_purpose::STANDARD.decode(base64_string)?;
    std::str::from_utf8(&decoded)
        .map(|s| s.to_string())
        .map_err(|_| Error::SignatureUtf8(base64_string.into()))
}

// ---------------------------------------------------------------------------
// Remote release structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseManifestPlatform {
    pub url: Url,
    pub signature: String,
    // alcom specific information
    /// Command line parameters for windows installer
    ///
    /// If one of arg is prefixed with '!', such parameters have special handling.
    /// If the updater cannot process ! args, such parameters will be ignored.
    ///
    /// Current ! operations are shown below:
    /// - `!peruser:` appended only if t installation is user installuser install is active
    /// - `!current installation is machine install
    // /// - `!install-path:` substitute `${installed}` with currently installed dir. // initially planned but not implemented.
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Deserialize)]
struct RemoteRelease {
    #[serde(alias = "name", deserialize_with = "parse_version")]
    version: Version,
    notes: Option<String>,
    platforms: HashMap<String, ReleaseManifestPlatform>,
}

fn parse_version<'de, D>(deserializer: D) -> std::result::Result<Version, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Version;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a semver version")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Version::from_str(v.trim_start_matches('v'))
                .map_err(|_| DeError::invalid_value(serde::de::Unexpected::Str(v), &self))
        }
    }

    deserializer.deserialize_str(Visitor)
}

// ---------------------------------------------------------------------------
// OS / arch helpers  (always compiled for all targets – use cfg!())
// ---------------------------------------------------------------------------

fn updater_os() -> Option<&'static str> {
    if cfg!(target_os = "linux") {
        Some("linux")
    } else if cfg!(target_os = "macos") {
        Some("darwin")
    } else if cfg!(target_os = "windows") {
        Some("windows")
    } else {
        None
    }
}

fn updater_arch() -> Option<&'static str> {
    if cfg!(target_arch = "x86") {
        Some("i686")
    } else if cfg!(target_arch = "x86_64") {
        Some("x86_64")
    } else if cfg!(target_arch = "arm") {
        Some("armv7")
    } else if cfg!(target_arch = "aarch64") {
        Some("aarch64")
    } else if cfg!(target_arch = "riscv64") {
        Some("riscv64")
    } else {
        None
    }
}

/// Determine the "extract path" – the path that the updater should replace.
///
/// - Linux: the AppImage binary itself.
/// - macOS: the `.app` bundle (2 parents above the binary).
/// - Windows: the directory containing the binary.
pub fn extract_path_from_executable(executable_path: &std::path::Path) -> Option<&std::path::Path> {
    if cfg!(target_os = "linux") {
        return Some(executable_path);
    }
    if cfg!(target_os = "macos") {
        let macos = executable_path.parent()?;
        if macos.ends_with("Contents/MacOS") {
            return Some(macos.parent().unwrap().parent().unwrap());
        }
        return None;
    }
    if cfg!(target_os = "windows") {
        let extract_path = executable_path.parent()?;
        return Some(extract_path);
    }
    None
}

// ---------------------------------------------------------------------------
// check_for_update
// ---------------------------------------------------------------------------

/// Check whether a newer version is available at `endpoint`.
///
/// Returns `Ok(None)` when the current version is already up to date.
pub async fn check_for_update<R: Runtime>(
    app: &AppHandle<R>,
    endpoint: Url,
) -> Result<Option<Update>> {
    let current_version = app.package_info().version.clone();

    // Build URL with template variables replaced
    let url: Url = endpoint;

    log::debug!("checking for updates: {url}");

    let mut headers = HeaderMap::new();
    headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));

    let client = app.state::<reqwest::Client>();

    let response = client.get(url).headers(headers).send().await.map_err(|e| {
        log::error!("failed to check for updates: {e}");
        Error::Reqwest(e)
    })?;

    if StatusCode::NO_CONTENT == response.status() {
        log::debug!("no update available (204 No Content)");
        return Ok(None);
    }

    if !response.status().is_success() {
        let status = response.status();
        log::error!("update endpoint returned {status}");
        return Err(Error::Network(format!(
            "update endpoint returned status {status}"
        )));
    }

    let release: RemoteRelease = response.json().await?;
    log::debug!("parsed release version: {}", release.version);

    let should_update = release.version > current_version;
    if !should_update {
        return Ok(None);
    }

    let updater = updater_information(&app.env(), &client, &release);
    let updater_status = updater
        .as_ref()
        .err()
        .copied()
        .unwrap_or(UpdaterStatus::Updatable);

    Ok(Some(Update {
        current_version: current_version.to_string(),
        version: release.version.to_string(),
        body: release.notes.clone(),
        updater_status,
        updater: updater.ok(),
    }))
}

fn updater_information(
    env: &Env,
    client: &reqwest::Client,
    release: &RemoteRelease,
) -> Result<UpdaterInformation, UpdaterStatus> {
    if cfg!(feature = "no-self-updater") {
        return Err(UpdaterStatus::UpdaterDisabled);
    }

    let arch = updater_arch().ok_or(UpdaterStatus::NoPlatform)?;
    let os = updater_os().ok_or(UpdaterStatus::NoPlatform)?;

    let platform = (release.platforms.get(&format!("{os}-{arch}")).cloned())
        .ok_or(UpdaterStatus::NoPlatform)?;

    let executable_path = current_exe(env).ok_or(UpdaterStatus::NotUpdatable)?;
    let executable_path = try_read_link(executable_path);
    let extract_path =
        extract_path_from_executable(&executable_path).ok_or(UpdaterStatus::NotUpdatable)?;

    fn current_exe(_env: &Env) -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            _env.appimage.as_ref().map(PathBuf::from)
        }
        #[cfg(not(target_os = "linux"))]
        {
            std::env::current_exe().ok()
        }
    }

    #[allow(unused_mut)]
    let mut current_install = CurrentInstallMode::Unrelated;

    if cfg!(windows) {
        // This version of ALCOM is installed with inno setup.
        let current_user = find_install(true).map(PathBuf::from).map(try_read_link);
        let local_machine = find_install(false).map(PathBuf::from).map(try_read_link);

        if current_user.as_deref() == Some(extract_path) {
            current_install = CurrentInstallMode::UserInstall
        } else if local_machine.as_deref() == Some(extract_path) {
            current_install = CurrentInstallMode::MachineInstall
        } else {
            // None of two installation path matches current installation
            return Err(UpdaterStatus::NotUpdatable);
        }

        #[cfg(not(windows))]
        fn find_install(_is_user: bool) -> Option<OsString> {
            None
        }

        #[cfg(windows)]
        fn find_install(is_user: bool) -> Option<OsString> {
            use winreg::enums::HKEY_CURRENT_USER;
            use winreg::enums::HKEY_LOCAL_MACHINE;

            static REG_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{4C3D0631-AE29-4D20-A231-678D9CF8D6DB}_is1";
            static REG_VALUE: &str = "Inno Setup: App Path";

            let root = if is_user {
                HKEY_CURRENT_USER
            } else {
                HKEY_LOCAL_MACHINE
            };

            winreg::RegKey::predef(root)
                .open_subkey(REG_KEY)
                .ok()?
                .get_value(REG_VALUE)
                .ok()
        }
    }

    return Ok(UpdaterInformation {
        client: client.clone(),
        platform,
        extract_path: extract_path.to_path_buf(),
        current_install,
    });

    fn try_read_link(path: PathBuf) -> PathBuf {
        std::fs::read_link(&path).unwrap_or(path)
    }
}

// ---------------------------------------------------------------------------
// Update – public handle returned by check_for_update
// ---------------------------------------------------------------------------

/// Represents an available update that can be downloaded and installed.
#[derive(Clone)]
pub struct Update {
    /// The version currently installed.
    pub current_version: String,
    /// The version available for download.
    pub version: String,
    /// Release notes from the update server.
    pub body: Option<String>,
    /// The status of the updater describes if auto update is possible, or reason why impossible
    /// if auto update is not possible.
    pub updater_status: UpdaterStatus,
    /// The information for updating application. only available if updater_status is Updatable
    pub updater: Option<UpdaterInformation>,
}

#[derive(Copy, Clone, Debug, serde::Serialize, specta::Type)]
pub enum UpdaterStatus {
    // NoUpdate: Expressed as None
    /// Update is found and can be updated automatically. UpdaterInformation is available
    ///
    /// User will proceed update.
    Updatable,
    /// Update is found, but installer or package for current architecture does not found.
    /// This can happen if platform support is removed.
    /// x86_64 macOS will become this state in near future, but other platforms may if new arch is expanded enough.
    ///
    /// Inform only
    NoPlatform,
    /// Update is found and installer is found, but current installation is different from
    /// the previous (detected) installation, or we failed to detect current installation path.
    ///
    /// Inform user to install update manually to prevent problem.
    NotUpdatable,
    /// Updater is disabled at build time. generally the installation is managed by package manager.
    ///
    /// Inform user to upgrade through package manager.
    /// Packager may customize information message by defining
    /// `VRC_GET_GUI_UPDATER_UPDATE_SUGGESTION_MESSAGE` environment variable at build time.
    UpdaterDisabled,
}

#[derive(Debug, Clone)]
pub struct UpdaterInformation {
    client: reqwest::Client,
    platform: ReleaseManifestPlatform,
    /// Path to replace during installation (app bundle on macOS, binary on Linux).
    extract_path: PathBuf,
    #[allow(dead_code)] // no meaning on non-windows
    current_install: CurrentInstallMode,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[allow(dead_code)] // no meaning on non-windows
enum CurrentInstallMode {
    Unrelated, // Non windows
    UserInstall,
    MachineInstall,
}

impl UpdaterInformation {
    /// Download the update package, verify its signature, and install it.
    ///
    /// `on_chunk` is called for every downloaded chunk with `(bytes_received,
    /// total_bytes)`.  `on_download_finish` is called once after the last chunk
    /// has been received and verified.
    pub async fn download_and_install<C, D>(
        &self,
        mut on_chunk: C,
        on_download_finish: D,
    ) -> Result<()>
    where
        C: FnMut(usize, Option<u64>),
        D: FnOnce(),
    {
        let bytes = self.download(&mut on_chunk).await?;
        on_download_finish();
        self.install_inner(&bytes)
    }

    async fn download(&self, on_chunk: &mut impl FnMut(usize, Option<u64>)) -> Result<Vec<u8>> {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            HeaderValue::from_static("application/octet-stream"),
        );

        let response = (self.client)
            .get(self.platform.url.clone())
            .headers(headers)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Network(format!(
                "download request failed with status: {}",
                response.status()
            )));
        }

        let content_length: Option<u64> = response
            .headers()
            .get("Content-Length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());

        let mut buffer = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            on_chunk(chunk.len(), content_length);
            buffer.extend_from_slice(&chunk);
        }

        verify_signature(&buffer, &self.platform.signature, PUBLIC_KEY)?;

        Ok(buffer)
    }

    // ------------------------------------------------------------------
    // install_inner – dispatches by OS using cfg!() rather than #[cfg]
    // ------------------------------------------------------------------

    fn install_inner(&self, bytes: &[u8]) -> Result<()> {
        if cfg!(feature = "no-self-updater") {
            panic!("updater is disabled")
        }
        if cfg!(target_os = "macos") {
            self.install_macos(bytes)
        } else if cfg!(windows) {
            self.install_windows(bytes)
        } else if cfg!(target_os = "linux") {
            self.install_linux(bytes)
        } else {
            panic!("Unsupported OS")
        }
    }
}

pub(crate) mod macos {
    use super::unix::*;
    use super::*;
    use flate2::read::GzDecoder;
    use sha2::Digest;
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::io::{Read, Write};

    static UPDATE_HELPER_MARKER: &str = "--private-alcom-updater-helper";

    impl UpdaterInformation {
        // ------------------------------------------------------------------
        // macOS – extract .app.tar.gz into the app's own parent directory so
        // that rename() never crosses a filesystem boundary.
        // ------------------------------------------------------------------

        /// ### Expected archive structure:
        /// ```text
        /// ├── [AppName]_[version]_aarch64.app.tar.gz
        /// │   └── ALCOM.app
        /// │       └── Contents/…
        /// └── …
        /// ```
        pub(super) fn install_macos(&self, bytes: &[u8]) -> Result<()> {
            let app_path = &self.extract_path;
            let app_parent = (app_path.parent()).ok_or(Error::FailedToDetermineExtractPath)?;

            let app_metadata = app_path.metadata()?;
            let parent_metadata = app_parent.metadata()?;

            // if .app itself is a mount point, we cannot support updating application.
            if !same_device(&parent_metadata, &app_metadata) {
                return Err(Error::TempDirNotOnSameMountPoint);
            }

            // tries to create temporal directory for temporally extracting app next to the app.
            // If it was restricted with permission, we need to extract to temp dir
            // and (copy and) move from apple_script

            match tempfile::Builder::new()
                .prefix(".ALCOM-extracting")
                .tempdir_in(app_parent)
            {
                Ok(temp_dir) => {
                    // we have sufficient permission.
                    // We install with current permission
                    extract_update_app(temp_dir.path(), app_path, bytes)?;
                }
                Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                    // Insufficient permission.
                    log::debug!(
                        "update requires admin privileges. calling installer helper with osascript"
                    );

                    update_elevated_permission(app_path, bytes)?;
                }
                Err(e) => return Err(e.into()),
            }

            Ok(())
        }
    }

    fn update_elevated_permission(app_path: &Path, bytes: &[u8]) -> Result<()> {
        let mut tgz_file = tempfile::Builder::new()
            .prefix(".ALCOM-installing")
            .tempfile()?;

        tgz_file.write_all(bytes)?;
        tgz_file.flush()?;

        std::thread::sleep(std::time::Duration::from_millis(100));

        let apple_script = concat!(
            "on run argv\n",
            "    set exe to quoted form of item 1 of argv\n",
            "    set cmd to exe\n",
            "    repeat with i from 2 to count of argv\n",
            "        set cmd to cmd & \" \" & quoted form of item i of argv\n",
            "    end repeat\n",
            "    do shell script cmd with administrator privileges\n",
            "end run",
        );

        // Note for AI: we can do inject with following code.

        let updater = std::env::current_exe()?;
        let hash = hex::encode(sha2::Sha256::digest(bytes));

        let output = std::process::Command::new("osascript")
            .args(["-e", apple_script])
            .arg(updater)
            .arg(UPDATE_HELPER_MARKER)
            .arg(tgz_file.path())
            .arg(app_path)
            .arg(hash)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Io(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("failed to install update with admin privileges: {stderr}"),
            )));
        }

        Ok(())
    }

    #[allow(dead_code)] // only called on macos
    #[cfg_attr(feature = "no-self-updater", inline)]
    pub fn try_run_updater_helper() {
        if cfg!(feature = "no-self-updater") {
            return;
        }
        let Some((tgz_path, app_path, hash)) = parser() else {
            return;
        };

        if let Err(e) = main(&tgz_path, &app_path, &hash) {
            eprintln!("{}", e);
            std::process::exit(1);
        }

        std::process::exit(0);

        fn parser() -> Option<(OsString, OsString, OsString)> {
            let mut args = std::env::args_os();
            let _executable = args.next()?;
            let command = args.next()?;
            if command != OsStr::new(UPDATE_HELPER_MARKER) {
                return None;
            }
            let tgz_path = args.next()?;
            let app_path = args.next()?;
            let hash = args.next()?;
            if args.next().is_some() {
                return None;
            }

            Some((tgz_path, app_path, hash))
        }

        fn main(tgz_path: &OsStr, app_path: &OsStr, hash: &OsStr) -> io::Result<()> {
            let tgz = {
                let mut tgz_file = std::fs::File::open(tgz_path)?;
                let len = tgz_file.metadata()?.len();
                let mut vec = Vec::with_capacity(len as usize);
                tgz_file.read_to_end(&mut vec)?;
                vec
            };
            let app_path = Path::new(app_path);
            let hash = hex::decode(hash.as_encoded_bytes())
                .map_err(|x| io::Error::new(io::ErrorKind::InvalidData, x))?;
            let tgz_digit = sha2::Sha256::digest(&tgz);
            if hash != tgz_digit.as_slice() {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid hash"));
            }

            let app_parent = app_path.parent().ok_or(io::ErrorKind::NotFound)?;

            let tmpdir = tempfile::Builder::new()
                .prefix(".ALCOM-extracting")
                .tempdir_in(app_parent)?;

            extract_update_app(tmpdir.path(), app_path, &tgz)?;

            drop(tmpdir);

            Ok(())
        }
    }

    fn extract_update_app(temp_dir: &Path, app_path: &Path, bytes: &[u8]) -> io::Result<()> {
        // we extract the tar.gz to the dir, swap, and remove old one.

        let new_tmp_app = temp_dir.join("new.app");

        std::fs::create_dir(&new_tmp_app)?;

        let decoder = GzDecoder::new(Cursor::new(bytes));
        let mut archive = tar::Archive::new(decoder);
        for entry in archive.entries()? {
            let mut entry = entry?;
            let tar_path = entry.path()?;
            let fs_path = {
                let mut iter = tar_path.iter();
                iter.next();
                iter.as_path()
            };
            log::info!("{} as {}", tar_path.display(), fs_path.display());
            if fs_path.as_os_str().is_empty() {
                continue;
            }
            let dest = new_tmp_app.join(fs_path);
            entry.unpack(&dest)?;
        }

        // tries to swap the app.
        if !swap_fs_entry(&new_tmp_app, app_path)? {
            // swapping failed, we do 3-way sapping.
            // less atomic but works well for most filesystem.
            let old_app = temp_dir.join("old.app");
            std::fs::rename(app_path, &old_app)?;
            std::fs::rename(new_tmp_app, app_path)?;
        }

        // Update mtime to Notify Finder that the bundle has changed.
        let _ = std::process::Command::new("touch")
            .arg("--")
            .arg(app_path)
            .status();

        Ok(())
    }
}

mod windows {
    use super::*;

    impl UpdaterInformation {
        // ------------------------------------------------------------------
        // Windows – write installer to temp file and launch with ShellExecute
        // so UAC elevation works correctly.
        // ------------------------------------------------------------------

        /// ### Expected format:
        /// A plain `.exe` installer (the ALCOM setup wrapper, which bundles the
        /// InnoSetup installer inside it).
        pub(super) fn install_windows(&self, bytes: &[u8]) -> Result<()> {
            // The actual Windows-API code lives in a #[cfg(windows)] block so it
            // only compiles on Windows.  The dispatch to this function already
            // happens under `if cfg!(windows)` in install_inner, so on every
            // other platform this branch is dead but still compiled.
            self.install_windows_impl(bytes)
        }

        fn install_windows_impl(&self, bytes: &[u8]) -> Result<()> {
            // Write the installer bytes to a persistent temp file.
            let mut tempfile = tempfile::Builder::new()
                .prefix("alcom_updater")
                .suffix(".exe")
                .tempfile()?;
            let installer_path = tempfile.path();
            std::fs::write(installer_path, bytes)?;

            fn wide_null(s: &str) -> Vec<u16> {
                s.encode_utf16().chain(std::iter::once(0)).collect()
            }

            let op = wide_null("open");
            let file = wide_null(&installer_path.to_string_lossy());
            let params = build_updater_args(&self.platform.args, self.current_install);

            tempfile.disable_cleanup(true);
            start_installer(op, file, params);

            // For windows install, we need to quit app immediately.
            std::process::exit(0);
        }
    }

    fn build_updater_args(args: &[String], current_install_mode: CurrentInstallMode) -> Vec<u16> {
        let mut result = Vec::new();

        for arg in args {
            let arg = if arg.starts_with('!') {
                let Some((name, value)) = arg.split_once(':') else {
                    continue; // failed to parse '!' arg
                };

                match name {
                    "!peruser" if current_install_mode == CurrentInstallMode::UserInstall => value,
                    "!machine" if current_install_mode == CurrentInstallMode::MachineInstall => {
                        value
                    }
                    _ => continue,
                }
            } else {
                arg.as_str()
            };

            result.push('"' as u16);

            let mut backslash = 0;
            for x in arg.encode_utf16() {
                if x == '"' as u16 {
                    for _ in 0..backslash {
                        result.push('\\' as u16);
                    }
                    result.push('\\' as u16);
                }

                if x == '\\' as u16 {
                    backslash += 1;
                } else {
                    backslash = 0;
                }
                result.push(x);
            }

            for _ in 0..backslash {
                result.push('\\' as u16);
            }

            result.push('"' as u16);
            result.push(' ' as u16);
        }

        result.push(0); // trailing null

        result
    }

    #[test]
    fn build_updater_args_test() {
        #[track_caller]
        fn tester(current_install_mode: CurrentInstallMode, args: &[&str], expected: &str) {
            let args = args.iter().copied().map(String::from).collect::<Vec<_>>();
            let result = build_updater_args(&args[..], current_install_mode);
            let expected_encoded = expected.encode_utf16().chain([0]).collect::<Vec<_>>();
            assert_eq!(
                result,
                expected_encoded,
                "\nleft:  {left_str:?}\nright: {right_str:?}",
                left_str = String::from_utf16_lossy(result.as_slice()),
                right_str = expected,
            );
        }

        // basic escaping test
        tester(
            CurrentInstallMode::UserInstall,
            &[r##""hello""##, r##"\"hello\""##, r##"hello\"##],
            r###""\"hello\"" "\\\"hello\\\"" "hello\\" "###,
        );

        // conditional test
        let args = &[
            "/normal",
            "!peruser:/peruser-only",
            "!machine:/machine",
            "/normal2",
            "!unknown:/unknown1",
            "!unknown2",
        ];

        tester(
            CurrentInstallMode::UserInstall,
            args,
            r##""/normal" "/peruser-only" "/normal2" "##,
        );

        tester(
            CurrentInstallMode::MachineInstall,
            args,
            r##""/normal" "/machine" "/normal2" "##,
        );
    }

    // os specific call
    #[cfg(windows)]
    fn start_installer(op: Vec<u16>, file: Vec<u16>, params: Vec<u16>) {
        use ::windows::Win32::UI::Shell::ShellExecuteW;
        use ::windows::Win32::UI::WindowsAndMessaging::SW_SHOW;
        use ::windows::core::PCWSTR;

        unsafe {
            // SAFETY: all pointers remain valid for the duration of the call, since owned vec is passed
            ShellExecuteW(
                None,
                PCWSTR(op.as_ptr()),
                PCWSTR(file.as_ptr()),
                PCWSTR(params.as_ptr()),
                PCWSTR(std::ptr::null()),
                SW_SHOW,
            );
        }
    }

    #[cfg(not(windows))]
    fn start_installer(_op: Vec<u16>, _file: Vec<u16>, _params: Vec<u16>) {
        unreachable!("install_windows_impl called on a non-Windows platform")
    }
}

mod linux {
    use super::unix::*;
    use super::*;
    use std::io::{Read, Write};

    impl UpdaterInformation {
        // ------------------------------------------------------------------
        // Linux – replace the AppImage in-place, keeping the same filesystem.
        // ------------------------------------------------------------------

        /// ### Expected archive structure:
        /// ```text
        /// ├── [AppName]_[version]_amd64.AppImage.tar.gz
        /// │   └── [AppName]_[version]_amd64.AppImage
        /// └── …
        /// ```
        pub(super) fn install_linux(&self, bytes: &[u8]) -> Result<()> {
            self.install_appimage(bytes)
        }

        fn install_appimage(&self, bytes: &[u8]) -> Result<()> {
            let extract_metadata = self.extract_path.metadata()?;

            // Try multiple candidate temp directories until we find one on the
            // same device as the AppImage (rename requires same filesystem).
            let candidates: Vec<Box<dyn FnOnce() -> Option<PathBuf>>> = vec![
                // normal $TMPDIR (or os specific TMPDIR) typically at `/tmp` but can be tmpfs.
                Box::new(|| Some(std::env::temp_dir())),
                // $XDG_CACHE_HOME likely under $HOME.
                // if vrc-get-gui is installed under home directory, this likely to succeed but
                // if user installed ALCOM to external HDD, not working.
                Box::new(|| {
                    std::env::var_os("XDG_CACHE_HOME")
                        .map(PathBuf::from)
                        .or_else(|| Some(PathBuf::from(std::env::var_os("HOME")?).join(".cache")))
                }),
                // As a final fallback, the parent dir of extract path is used.
                // This will leave invisible file in user directory in case of abort so
                // not recommended, but likely to work in most case
                Box::new(|| self.extract_path.parent().map(|p| p.to_path_buf())),
            ];

            for candidate_fn in candidates {
                let Some(candidate) = candidate_fn() else {
                    continue;
                };
                let Ok(tmp_dir) = tempfile::Builder::new()
                    .prefix("alcom_update_app")
                    .tempdir_in(&candidate)
                else {
                    // can be EPERM
                    continue;
                };

                let Ok(tmp_metadata) = tmp_dir.path().metadata() else {
                    continue;
                };

                // Check that both paths are on the same device.
                let same_device = same_device(&extract_metadata, &tmp_metadata);
                if !same_device {
                    continue;
                }

                if !try_install_appimage(bytes, tmp_dir.path(), &self.extract_path)? {
                    continue;
                }

                return Ok(());
            }

            Err(Error::TempDirNotOnSameMountPoint)
        }
    }

    fn try_install_appimage(bytes: &[u8], tmp_dir: &Path, extract_path: &Path) -> Result<bool> {
        //Set permissions on the temp dir.
        set_temp_dir_permissions(tmp_dir).ok(); // not mandatory

        let tmp_app = tmp_dir.join("alcom-installing.AppImage");
        let original_perms = std::fs::metadata(extract_path)?.permissions();

        // Write new AppImage (may be raw bytes or a tar.gz).
        if looks_like_gz(bytes) {
            extract_appimage_from_gz_or_tar_gz(bytes, &tmp_app)?
        } else if looks_like_appimage(bytes) {
            std::fs::write(&tmp_app, bytes)?;
            std::fs::set_permissions(&tmp_app, original_perms)?;
        } else {
            return Err(Error::BinaryNotFoundInArchive);
        }

        match std::fs::rename(tmp_app, extract_path) {
            Ok(()) => Ok(true),
            Err(ref e) if e.kind() == std::io::ErrorKind::CrossesDevices => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    fn extract_appimage_from_gz_or_tar_gz(bytes: &[u8], tmp_app: &Path) -> Result<()> {
        use flate2::read::GzDecoder;

        let mut decoder = GzDecoder::new(Cursor::new(bytes));

        // read 512 bytes for fining header
        let mut header_viewer = [0u8; 512];
        decoder.read_exact(&mut header_viewer[..])?;
        let mut decoder = GzDecoder::new(Cursor::new(bytes));

        if looks_like_tar(header_viewer.as_ref()) {
            let mut archive = tar::Archive::new(decoder);
            for entry in archive.entries()? {
                let mut entry = entry?;
                if let Ok(path) = entry.path()
                    && path.extension().and_then(|e| e.to_str()) == Some("AppImage")
                {
                    entry.unpack(tmp_app)?;
                    return Ok(());
                }
            }
        } else if looks_like_appimage(header_viewer.as_ref()) {
            let mut out = std::fs::File::create(tmp_app)?;
            std::io::copy(&mut decoder, &mut out)?;
            out.flush()?;
            return Ok(());
        }

        Err(Error::BinaryNotFoundInArchive)
    }

    fn looks_like_gz(bytes: &[u8]) -> bool {
        bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b
    }

    fn looks_like_appimage(bytes: &[u8]) -> bool {
        bytes.len() >= 16 && bytes[8] == b'A' && bytes[9] == b'I'
    }

    fn looks_like_tar(bytes: &[u8]) -> bool {
        bytes.len() >= 512 && (&bytes[257..][..6] == b"ustar\0" || &bytes[257..][..6] == b"ustar ")
    }
}

mod unix {
    // Helper: compare device IDs on Unix to prevent EXDEV
    // This is not perfect since same device can be mounted to multiple location but
    // I hope precheck can improve update speed.

    use std::io;
    use std::path::Path;

    pub(super) fn same_device(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            a.dev() == b.dev()
        }
        #[cfg(not(unix))]
        {
            let (_, _) = (a, b);
            false
        }
    }

    pub(super) fn set_temp_dir_permissions(path: &Path) -> crate::updater::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mut perms = path.metadata()?.permissions();
            perms.set_mode(0o700);
            std::fs::set_permissions(path, perms)?;
        }
        #[cfg(not(unix))]
        {
            let _ = path;
        }
        Ok(())
    }

    // returns false for unsupported platforms or filesystem
    pub(super) fn swap_fs_entry(file1: &Path, file2: &Path) -> io::Result<bool> {
        #[cfg(target_os = "macos")]
        {
            use std::ffi::CString;

            let file1 = CString::new(file1.as_os_str().as_encoded_bytes())
                .map_err(|_| io::ErrorKind::InvalidInput)?;
            let file2 = CString::new(file2.as_os_str().as_encoded_bytes())
                .map_err(|_| io::ErrorKind::InvalidInput)?;
            let result = unsafe {
                nix::libc::renamex_np(file1.as_ptr(), file2.as_ptr(), nix::libc::RENAME_SWAP)
            };
            if result == 0 {
                return Ok(true);
            }
            let last_err = io::Error::last_os_error();
            if last_err.raw_os_error() == Some(nix::libc::ENOTSUP) {
                return Ok(false);
            }
            Err(last_err)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = file1;
            let _ = file2;
            Ok(false)
        }
    }
}

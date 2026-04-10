use crate::utils::command::{CommandExt, WineRunner};
use crate::utils::rustc::rustc_host_triple;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

const WEBVIEW2_URL: &str = "https://go.microsoft.com/fwlink/?linkid=2124703";
const INNO_SETUP_VERSION: &str = "6.7.1";
const INNO_SETUP_COMPILER_FILE: &str = "ISCC.exe";
const INNO_SETUP_INSTALLER_URL: &str =
    "https://github.com/jrsoftware/issrc/releases/download/is-6_7_1/innosetup-6.7.1.exe";
const EMPTY_LIBS_DIR: &str = "empty-libs";
const INSTALLER_NAME: &str = "alcom-inno-setup.exe";
const DEFAULT_VERSION: &str = "1.1.5";
const TARGET_CANDIDATES: [&str; 3] = [
    "x86_64-pc-windows-msvc",
    "x86_64-pc-windows-gnu",
    "x86_64-pc-windows-gnullvm",
];

/// Builds alcom inno-setup based installer with NSIS compat layer
#[derive(clap::Parser)]
pub(super) struct Command {
    /// Target triple for windows-installer-wrapper and alcom
    ///
    /// This target will also be used to find ALCOM.exe.
    #[arg(long)]
    target: Option<String>,

    /// Application version injected into installer metadata.
    #[arg(long, default_value = DEFAULT_VERSION)]
    version: String,

    /// Profile for the build
    ///
    /// ALCOM.exe should be built with this profile and wrapper will be built for this profile
    #[arg(long, default_value = "release")]
    profile: String,
}

impl crate::Command for Command {
    fn run(self) -> Result<i32> {
        let metadata = crate::utils::cargo::cargo_metadata();
        let workspace_root = metadata.workspace_root.as_std_path();
        let target_dir = metadata.target_directory.as_std_path();

        let profile = "release";
        let host_triple = rustc_host_triple()?;
        let target_triple = (self.target.as_deref()).or_else(|| choose_default_target(host_triple));

        let build_dir = if let Some(target) = target_triple {
            target_dir.join(target).join(profile)
        } else {
            target_dir.join(profile)
        };

        let inno_setup_base = target_dir.join("inno-setup");
        let inno_setup = inno_setup_base.join(INNO_SETUP_VERSION);

        let installer_build = build_dir.join("installer");
        let alcom_exe = build_dir.join("ALCOM.exe");

        let license_path = workspace_root.join("LICENSE");
        let installer_script = workspace_root.join("vrc-get-gui/installer.iss");

        let options = BuildOptions {
            target_triple,
            host_triple,
            build_target: target_triple.unwrap_or(host_triple),

            app_version: &self.version,

            workspace_root,
            target_dir,
            build_dir: &build_dir,

            license_path: &license_path,
            alcom_exe: &alcom_exe,
            installer_script: &installer_script,

            inno_setup_dir: &inno_setup,
            installer_build: &installer_build,
        };

        let runner = WineRunner::detect()?;
        install_inno_setup(&options, &runner)?;
        build_inno_setup_installer(&options, &runner)?;

        let libs_dir = create_empty_libs(&options)?;

        let wrapper_exe = build_wrapper(&options, &libs_dir, profile)?;
        let is_static = crate::check_static_link::check_static_link(&wrapper_exe)
            .with_context(|| format!("checking static link: {}", wrapper_exe.display()))?;

        if !is_static {
            eprintln!(
                "non-OS dynamic dependencies found in {}",
                wrapper_exe.display()
            );
            return Ok(1);
        }

        println!("{}", wrapper_exe.display());
        Ok(0)
    }
}

struct BuildOptions<'a> {
    // environment information
    target_triple: Option<&'a str>,
    #[allow(dead_code)]
    host_triple: &'a str,
    build_target: &'a str, // = target_triple.unwrap_or(host_triple)

    // configuration
    app_version: &'a str,

    // input files
    license_path: &'a Path,
    alcom_exe: &'a Path,
    installer_script: &'a Path,

    // path information
    workspace_root: &'a Path,
    #[allow(dead_code)]
    target_dir: &'a Path, // the base dir for all build cache
    build_dir: &'a Path, // the base dir for target & profile dependant data

    inno_setup_dir: &'a Path,  // The path inno setup was installed.
    installer_build: &'a Path, // the base dir for building inno setup installer
}

fn download_file(url: &str, dest: &Path, what: &str) -> Result<()> {
    fs::create_dir_all(dest.parent().unwrap())?;

    let mut response = crate::utils::ureq()
        .get(url)
        .call()
        .context(format!("{what}: downloading {url}"))?;

    std::io::copy(
        &mut response.body_mut().as_reader(),
        &mut fs::File::create(dest).context(format!("{what}: creating file"))?,
    )
    .context(format!("{what}: saving {url}"))?;
    Ok(())
}

fn install_inno_setup(options: &BuildOptions, runner: &WineRunner) -> Result<()> {
    let iscc = options.inno_setup_dir.join(INNO_SETUP_COMPILER_FILE);
    if iscc.is_file() {
        // skip if alrady installed
        return Ok(());
    }

    fs::create_dir_all(options.inno_setup_dir)
        .with_context(|| format!("creating {}", options.inno_setup_dir.display()))?;

    let installer_exe = options
        .inno_setup_dir
        .join(format!("innosetup-installer-{INNO_SETUP_VERSION}.exe"));

    download_file(
        INNO_SETUP_INSTALLER_URL,
        &installer_exe,
        "downloading Inno Setup installer",
    )?;

    let mut cmd = runner.command(&installer_exe);

    cmd.arg("/SP-")
        .arg("/VERYSILENT")
        .arg("/SUPPRESSMSGBOXES")
        .arg("/NORESTART")
        .arg("/CURRENTUSER")
        .arg(format!("/DIR={}", runner.path(options.inno_setup_dir)));

    cmd.run_checked("installing Inno Setup")?;

    if !iscc.is_file() {
        bail!("Inno Setup installation did not produce {}", iscc.display());
    }

    Ok(())
}

fn build_inno_setup_installer(options: &BuildOptions, runner: &WineRunner) -> Result<()> {
    let webview2_installer = options
        .installer_build
        .join("deps/MicrosoftEdgeWebView2Setup.exe");

    download_file(
        WEBVIEW2_URL,
        &webview2_installer,
        "downloading WebView2 bootstrapper",
    )?;

    let webview2 = runner.path(&webview2_installer);
    let license = runner.path(options.license_path);
    let app_path = runner.path(options.alcom_exe);
    let version = options.app_version;
    let mut cmd = runner.command(&options.inno_setup_dir.join(INNO_SETUP_COMPILER_FILE));

    cmd.arg(runner.path(options.installer_script))
        .arg(format!("-DWebView2SetupPath={webview2}"))
        .arg(format!("-DLicensePath={license}"))
        .arg(format!("-DApplicationVersion={version}"))
        .arg(format!("-DApplicationPath={app_path}"))
        .arg(format!(
            "-F{}",
            INSTALLER_NAME.strip_suffix(".exe").unwrap()
        ))
        .arg(format!("-O{}/", options.installer_build.display()));

    cmd.run_checked("running Inno Setup compiler")
}

fn create_empty_libs(options: &BuildOptions) -> Result<PathBuf> {
    static EMPTY_ARCHIVE: &[u8] = b"!<arch>\n";

    let libs_dir = options.installer_build.join(EMPTY_LIBS_DIR);
    fs::create_dir_all(&libs_dir).with_context(|| format!("creating {}", libs_dir.display()))?;

    for lib_name in [
        "msvcrt.lib",
        "libpthread.a",
        "libgcc_eh.a",
        "libmingwex.a",
        "libmingw32.a",
        "libgcc.a",
        "libmingwex.a",
        "libuser32.a",
        "libkernel32.a",
    ] {
        fs::write(libs_dir.join(lib_name), EMPTY_ARCHIVE)
            .with_context(|| format!("writing {} with empty ar archive", lib_name))?;
    }

    Ok(libs_dir)
}

fn build_wrapper(options: &BuildOptions, libs_dir: &Path, profile: &str) -> Result<PathBuf> {
    let rustflags = if options.build_target.ends_with("-msvc") {
        format!("-C link-arg=-LIBPATH:{}", libs_dir.display())
    } else {
        format!("-C link-arg=-L{}", libs_dir.display())
    };

    let mut cmd = ProcessCommand::new("cargo");
    cmd.current_dir(options.workspace_root)
        .arg("build")
        .arg("-p")
        .arg("windows-installer-wrapper");

    cmd.arg("--profile").arg(profile);

    if let Some(target) = options.target_triple {
        cmd.arg("--target").arg(target);
    }

    cmd.env("RUSTFLAGS", rustflags).env(
        "INSTALLER_EXE",
        options.installer_build.join(INSTALLER_NAME),
    );

    // prefer rust-lld for cross compoling
    let compile_target_env_name = options.build_target.to_ascii_uppercase().replace('-', "_");
    cmd.env(
        format!("CARGO_TARGET_{compile_target_env_name}_LINKER"),
        "rust-lld",
    );

    let profile_env_name = profile.to_ascii_uppercase().replace("-", "_");
    cmd.env(format!("CARGO_PROFILE_{profile_env_name}_LTO"), "fat")
        .env(format!("CARGO_PROFILE_{profile_env_name}_PANIC"), "abort")
        .env(format!("CARGO_PROFILE_{profile_env_name}_STRIP"), "symbols");

    cmd.run_checked("building windows-installer-wrapper")?;

    Ok(options.build_dir.join("alcom-setup.exe"))
}

fn choose_default_target(host_triple: &str) -> Option<&'static str> {
    // if host is windows, use the target
    if TARGET_CANDIDATES.contains(&host_triple) {
        return None;
    }

    let installed = installed_targets().unwrap_or_default();
    for candidate in TARGET_CANDIDATES {
        if installed.iter().any(|x| x == candidate) {
            return Some(candidate);
        }
    }

    Some(TARGET_CANDIDATES[0])
}

fn installed_targets() -> Result<Vec<String>> {
    let mut cmd = ProcessCommand::new("rustup");
    cmd.arg("target").arg("list").arg("--installed");
    let output = cmd.run_capture_checked("listing installed rust targets")?;

    Ok(output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

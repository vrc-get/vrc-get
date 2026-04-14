use crate::bundle_alcom::BundleContext;
use crate::utils::command::{CommandExt, WineRunner};
use crate::utils::{download_file_cached, target_abi};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

const WEBVIEW2_URL: &str = "https://go.microsoft.com/fwlink/?linkid=2124703";
const INNO_SETUP_VERSION: &str = "6.7.1";
const INNO_SETUP_INSTALLER_URL: &str =
    "https://github.com/jrsoftware/issrc/releases/download/is-6_7_1/innosetup-6.7.1.exe";

pub fn create_setup_exe(ctx: &BundleContext<'_>) -> Result<()> {
    let runner = WineRunner::detect()?;

    let iscc = install_inno_setup(ctx, &runner)?;
    let webview_bootstrapper = download_webview2_bootstrapper(ctx)?;

    let iss_setup = build_inno_setup_installer(ctx, &iscc, &webview_bootstrapper, &runner)?;

    // copy to bundle dir
    let wrapper_in_bundle = ctx.bundle_dir.join("setup/alcom-setup.exe");
    fs::copy(&iss_setup, &wrapper_in_bundle)
        .with_context(|| format!("copying: {}", wrapper_in_bundle.display()))?;

    println!("created: {}", wrapper_in_bundle.display());
    Ok(())
}

pub fn create_updater_exe(ctx: &BundleContext<'_>) -> Result<()> {
    let iss_setup = ctx.bundle_dir.join("setup/alcom-setup.exe");

    let libs_dir = create_empty_libs(ctx)?;

    let wrapper_exe = build_wrapper(ctx, &libs_dir, &iss_setup)?;

    let is_static = crate::check_static_link::check_static_link(&wrapper_exe)
        .with_context(|| format!("checking static link: {}", wrapper_exe.display()))?;
    if !is_static {
        bail!(
            "non-OS dynamic dependencies found in {}",
            wrapper_exe.display()
        );
    }

    // copy to bundle dir
    let wrapper_in_bundle = ctx.bundle_dir.join("setup/alcom-updater.exe");
    fs::copy(&wrapper_exe, &wrapper_in_bundle)
        .with_context(|| format!("copying: {}", wrapper_in_bundle.display()))?;

    println!("created: {}", wrapper_in_bundle.display());
    Ok(())
}

fn install_inno_setup(ctx: &BundleContext<'_>, runner: &WineRunner) -> Result<PathBuf> {
    let inno_setup = ctx
        .host_build_dir
        .join("bundle/inno-setup")
        .join(INNO_SETUP_VERSION);
    let iscc = inno_setup.join("ISCC.exe");
    if iscc.is_file() {
        // skip if alrady installed
        return Ok(iscc);
    }

    fs::create_dir_all(&inno_setup)
        .with_context(|| format!("creating {}", inno_setup.display()))?;

    let installer_exe = inno_setup.join(format!("innosetup-installer-{INNO_SETUP_VERSION}.exe"));

    download_file_cached(
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
        .arg(format!("/DIR={}", runner.path(&inno_setup)));

    cmd.run_checked("installing Inno Setup")?;

    if !iscc.is_file() {
        bail!("Inno Setup installation did not produce {}", iscc.display());
    }

    Ok(iscc)
}

fn download_webview2_bootstrapper(ctx: &BundleContext<'_>) -> Result<PathBuf> {
    let webview2_installer = ctx
        .bundle_dir
        .join("setup/deps/MicrosoftEdgeWebView2Setup.exe");

    download_file_cached(
        WEBVIEW2_URL,
        &webview2_installer,
        "downloading WebView2 bootstrapper",
    )?;

    Ok(webview2_installer)
}

fn build_inno_setup_installer(
    ctx: &BundleContext<'_>,
    iscc: &Path,
    webview2_bootstrapper: &Path,
    runner: &WineRunner,
) -> Result<PathBuf> {
    let webview2 = runner.path(webview2_bootstrapper);
    let license = runner.path(&ctx.workspace_root.join("LICENSE"));
    let app_path = runner.path(&ctx.binary_path());
    let version = ctx.version();
    let mut cmd = runner.command(iscc);

    const INSTALLER_NAME: &str = "alcom-inno-setup";

    cmd.arg(runner.path(&ctx.gui_dir.join("bundle/windows-setup.iss")))
        .arg(format!("-DWebView2SetupPath={webview2}"))
        .arg(format!("-DLicensePath={license}"))
        .arg(format!("-DApplicationVersion={version}"))
        .arg(format!("-DApplicationPath={app_path}"))
        .arg(format!("-F{}", INSTALLER_NAME))
        .arg(format!(
            "-O{}/",
            ctx.bundle_dir.join("setup/deps/iss").display()
        ));

    cmd.run_checked("running Inno Setup compiler")?;

    Ok((ctx.bundle_dir)
        .join("setup/deps/iss")
        .join(INSTALLER_NAME)
        .with_extension("exe"))
}

fn create_empty_libs(ctx: &BundleContext) -> Result<PathBuf> {
    static EMPTY_ARCHIVE: &[u8] = b"!<arch>\n";

    let libs_dir = ctx.bundle_dir.join("setup/deps/libs");
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

fn build_wrapper(ctx: &BundleContext<'_>, libs_dir: &Path, iss_setup: &Path) -> Result<PathBuf> {
    let rustflags = if target_abi(ctx.target_tuple) == "msvc" {
        format!("-C link-arg=-LIBPATH:{}", libs_dir.display())
    } else {
        format!("-C link-arg=-L{}", libs_dir.display())
    };

    let mut cmd = ProcessCommand::new("cargo");
    cmd.current_dir(ctx.workspace_root)
        .arg("build")
        .arg("-p")
        .arg("windows-installer-wrapper");

    cmd.arg("--profile").arg(ctx.profile);

    if let Some(target) = ctx.target {
        cmd.arg("--target").arg(target);
    }

    cmd.env("RUSTFLAGS", rustflags)
        .env("INSTALLER_EXE", iss_setup);

    // prefer rust-lld for cross compoling
    let compile_target_env_name = ctx.target_tuple.to_ascii_uppercase().replace('-', "_");
    cmd.env(
        format!("CARGO_TARGET_{compile_target_env_name}_LINKER"),
        "rust-lld",
    );

    // minimize file by changing several build options
    let profile_env_name = ctx.profile.to_ascii_uppercase().replace("-", "_");
    cmd.env(format!("CARGO_PROFILE_{profile_env_name}_LTO"), "fat")
        .env(format!("CARGO_PROFILE_{profile_env_name}_PANIC"), "abort")
        .env(format!("CARGO_PROFILE_{profile_env_name}_STRIP"), "symbols");

    cmd.run_checked("building windows-installer-wrapper")?;

    Ok(ctx.build_dir.join("alcom-setup.exe"))
}

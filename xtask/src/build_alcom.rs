use crate::utils::command::CommandExt;
use crate::utils::rustc::rustc_host_triple;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;

/// Builds the ALCOM binary using `cargo build`.
///
/// This replaces `npm run tauri build --no-bundle`, removing the dependency on the
/// Node.js / npm toolchain for the binary compilation step.
///
/// When targeting `universal-apple-darwin`, both `aarch64-apple-darwin` and
/// `x86_64-apple-darwin` are compiled and then combined with `lipo`.
///
/// The `custom-protocol` feature is always enabled, which is required when calling
/// `cargo build` directly instead of through the tauri CLI.
#[derive(clap::Parser)]
pub(super) struct Command {
    /// Target triple (e.g. `universal-apple-darwin`, `x86_64-unknown-linux-gnu`).
    ///
    /// Defaults to the host triple.
    #[arg(long)]
    target: Option<String>,

    /// Build profile (default: `release`).
    #[arg(long, default_value = "release")]
    profile: String,

    /// Enable verbose cargo output.
    #[arg(long)]
    verbose: bool,
}

impl crate::Command for Command {
    fn run(self) -> Result<i32> {
        let metadata = crate::utils::cargo::cargo_metadata();
        let workspace_root = metadata.workspace_root.as_std_path();
        let target_dir = metadata.target_directory.as_std_path();

        let host_triple = rustc_host_triple()?;
        let target_triple = self.target.as_deref().unwrap_or(host_triple);

        if target_triple == "universal-apple-darwin" {
            build_universal_macos(workspace_root, target_dir, &self.profile, self.verbose)?;
        } else {
            build_cargo(workspace_root, target_triple, &self.profile, self.verbose)?;
        }

        Ok(0)
    }
}

/// Run `cargo build -p vrc-get-gui` for a single target triple.
fn build_cargo(
    workspace_root: &Path,
    target_triple: &str,
    profile: &str,
    verbose: bool,
) -> Result<()> {
    let mut cmd = ProcessCommand::new("cargo");
    cmd.current_dir(workspace_root)
        .arg("build")
        .arg("-p")
        .arg("vrc-get-gui")
        .arg("--features")
        .arg("custom-protocol")
        .arg("--target")
        .arg(target_triple)
        .arg("--profile")
        .arg(profile);

    if verbose {
        cmd.arg("--verbose");
    }

    cmd.run_checked(&format!("building vrc-get-gui for {target_triple}"))
}

/// Build a universal macOS binary by compiling for both x86_64 and aarch64 and
/// merging the results with `lipo`.
fn build_universal_macos(
    workspace_root: &Path,
    target_dir: &Path,
    profile: &str,
    verbose: bool,
) -> Result<()> {
    build_cargo(workspace_root, "x86_64-apple-darwin", profile, verbose)?;
    build_cargo(workspace_root, "aarch64-apple-darwin", profile, verbose)?;

    // Combine the two single-arch binaries into one fat binary.
    let x86_bin = target_dir
        .join("x86_64-apple-darwin")
        .join(profile)
        .join("ALCOM");
    let arm_bin = target_dir
        .join("aarch64-apple-darwin")
        .join(profile)
        .join("ALCOM");

    let out_dir = target_dir.join("universal-apple-darwin").join(profile);
    fs::create_dir_all(&out_dir).with_context(|| format!("creating {}", out_dir.display()))?;

    let out_bin = out_dir.join("ALCOM");

    lipo_create(&[&x86_bin, &arm_bin], &out_bin)?;

    println!("created universal binary: {}", out_bin.display());
    Ok(())
}

/// Create a universal binary from a list of single-arch binaries using `lipo`.
fn lipo_create(inputs: &[&Path], output: &Path) -> Result<()> {
    let mut cmd = ProcessCommand::new("lipo");
    cmd.arg("-create").arg("-output").arg(output);
    for input in inputs {
        cmd.arg(input);
    }
    cmd.run_checked("lipo: creating universal binary")
}

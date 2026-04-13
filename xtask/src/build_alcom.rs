use crate::utils;
use crate::utils::command::CommandExt;
use crate::utils::{build_dir, build_target};
use anyhow::{Context, Result};
use itertools::Itertools;
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

    #[command(flatten)]
    profile: utils::BuildProfile,

    #[arg(long, value_delimiter = ',')]
    features: Vec<String>,

    /// Enable verbose cargo output.
    #[arg(long)]
    verbose: bool,
}

impl crate::Command for Command {
    fn run(self) -> Result<i32> {
        let metadata = crate::utils::cargo::cargo_metadata();
        let workspace_root = metadata.workspace_root.as_std_path();

        let target_triple = build_target(self.target.as_deref());

        if target_triple == "universal-apple-darwin" {
            build_universal_macos(
                workspace_root,
                self.profile.name(),
                &self.features,
                self.verbose,
            )?;
        } else {
            build_cargo(
                workspace_root,
                self.target.as_deref(),
                self.profile.name(),
                &self.features,
                self.verbose,
            )?;
        }

        Ok(0)
    }
}

/// Run `cargo build -p vrc-get-gui` for a single target triple.
fn build_cargo(
    workspace_root: &Path,
    target_triple: Option<&str>,
    profile: &str,
    features: &[String],
    verbose: bool,
) -> Result<()> {
    let mut cmd = ProcessCommand::new("cargo");
    cmd.current_dir(workspace_root)
        .arg("build")
        .arg("-p")
        .arg("vrc-get-gui")
        .arg("--profile")
        .arg(profile);

    cmd.arg("--features").arg(
        features
            .iter()
            .map(AsRef::as_ref)
            .chain(["custom-protocol"])
            .join(","),
    );

    if let Some(target) = target_triple {
        cmd.arg("--target").arg(target);
    }

    if verbose {
        cmd.arg("--verbose");
    }

    cmd.run_checked(&format!(
        "building vrc-get-gui for {}",
        target_triple.unwrap_or("native target")
    ))
}

/// Build a universal macOS binary by compiling for both x86_64 and aarch64 and
/// merging the results with `lipo`.
fn build_universal_macos(
    workspace_root: &Path,
    profile: &str,
    features: &[String],
    verbose: bool,
) -> Result<()> {
    build_cargo(
        workspace_root,
        Some("x86_64-apple-darwin"),
        profile,
        features,
        verbose,
    )?;
    build_cargo(
        workspace_root,
        Some("aarch64-apple-darwin"),
        profile,
        features,
        verbose,
    )?;

    // Combine the two single-arch binaries into one fat binary.
    let x86_bin = build_dir("x86_64-apple-darwin", profile).join("ALCOM");
    let arm_bin = build_dir("aarch64-apple-darwin", profile).join("ALCOM");

    let out_dir = build_dir("universal-apple-darwin", profile);
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

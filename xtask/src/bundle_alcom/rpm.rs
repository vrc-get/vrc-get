use super::BundleContext;
use crate::bundle_alcom::linux::*;
use anyhow::{Context, Result};
use std::fs;

pub fn create_rpm(ctx: &BundleContext<'_>) -> Result<()> {
    let arch = rpm_arch(ctx.target_tuple);
    let rpm_name = format!("alcom-{}-1.{arch}.rpm", ctx.version());
    let rpm_dir = ctx.bundle_dir.join("rpm");
    fs::create_dir_all(&rpm_dir)?;
    let rpm_out = rpm_dir.join(&rpm_name);

    let mut builder = rpm::PackageBuilder::new(
        "alcom",
        // RPM doesn't support '-' in their version name.
        // It's recommended to use '~' instead.
        // https://docs.fedoraproject.org/en-US/packaging-guidelines/Versioning/#_handling_non_sorting_versions_with_tilde_dot_and_caret
        &ctx.version().replace('-', "~"),
        "MIT",
        arch,
        ctx.short_description(),
    );
    builder.release("1").description(ctx.long_description());

    // Binary.
    create_install_build_root_impl(ctx, &mut builder).context("adding files to rpm")?;

    let pkg = builder.build().context("building rpm package")?;

    pkg.write_file(&rpm_out)
        .with_context(|| format!("writing {}", rpm_out.display()))?;

    println!("created: {}", rpm_out.display());
    Ok(())
}

/// RPM architecture string.
fn rpm_arch(triple: &str) -> &str {
    if triple.starts_with("aarch64") {
        "aarch64"
    } else if triple.starts_with("x86_64") {
        "x86_64"
    } else {
        panic!(
            "unsupported architecture in target triple for rpm: {}",
            triple
        )
    }
}

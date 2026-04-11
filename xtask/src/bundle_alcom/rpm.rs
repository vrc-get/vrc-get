use super::BundleContext;
use crate::bundle_alcom::linux::*;
use anyhow::{Context, Result};
use std::fs;

pub fn create_rpm(ctx: &BundleContext<'_>) -> Result<()> {
    let arch = rpm_arch(ctx.target_truple);
    let rpm_name = format!("ALCOM-{}-1.{arch}.rpm", ctx.config.version);
    let rpm_dir = ctx.bundle_dir.join("rpm");
    fs::create_dir_all(&rpm_dir)?;
    let rpm_out = rpm_dir.join(&rpm_name);

    let bin_name_lower = ctx.binary_name().to_ascii_lowercase();

    // Write desktop file to a temp location so rpm::PackageBuilder can read it from disk.
    let desktop_content = render_desktop_file(ctx, &format!("/usr/bin/{bin_name_lower}"))?;
    let desktop_tmp = rpm_dir.join(format!("{bin_name_lower}.desktop.tmp"));
    fs::write(&desktop_tmp, &desktop_content)?;

    let mut builder = rpm::PackageBuilder::new(
        "alcom",
        &ctx.config.version,
        "MIT",
        arch,
        &ctx.config.short_description,
    )
    .release("1")
    .description(&ctx.config.long_description);

    // Binary.
    builder = builder
        .with_file(
            ctx.binary_path(),
            rpm::FileOptions::new(format!("/usr/bin/{bin_name_lower}")).mode(0o755),
        )
        .context("adding binary to rpm")?;

    // Desktop file.
    builder = builder
        .with_file(
            &desktop_tmp,
            rpm::FileOptions::new(format!("/usr/share/applications/{bin_name_lower}.desktop")),
        )
        .context("adding desktop file to rpm")?;

    // Icons.
    for size in LINUX_ICON_RESOLUTIONS {
        let install_path = format!("/usr/share/icons/hicolor/{size}/apps/alcom.png");
        builder = builder
            .with_file(ctx.icon_path(size), rpm::FileOptions::new(install_path))
            .with_context(|| format!("adding icon {size}.png to rpm"))?;
    }

    let pkg = builder.build().context("building rpm package")?;
    pkg.write_file(&rpm_out)
        .with_context(|| format!("writing {}", rpm_out.display()))?;

    // Clean up temp desktop file.
    let _ = fs::remove_file(&desktop_tmp);

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

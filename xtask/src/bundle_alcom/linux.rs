use super::BundleContext;
use anyhow::{Context, Result};
use std::fs;

/// Render the desktop file template from `alcom.desktop`.
///
/// The template uses `{{key}}` placeholders as in the tauri bundler.
pub fn render_desktop_file(ctx: &BundleContext<'_>, exec: &str) -> Result<String> {
    let template_path = ctx.gui_dir.join("bundle/alcom.desktop");
    let template = fs::read_to_string(&template_path)
        .with_context(|| format!("reading {}", template_path.display()))?;

    Ok(template.replace("{{exec}}", exec))
}

pub static LINUX_ICON_RESOLUTIONS: &[&str] =
    &["icons/32x32.png", "icons/64x64.png", "icons/128x128.png"];

pub static LINUX_ICON_NAME: &str = "alcom"; // keep in sync with alcom.desktop template

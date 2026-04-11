use super::BundleContext;
use anyhow::{Context, Result};
use std::fs;

/// Render the desktop file template from `alcom.desktop`.
///
/// The template uses `{{key}}` placeholders as in the tauri bundler.
pub fn render_desktop_file(ctx: &BundleContext<'_>, exec: &str) -> Result<String> {
    let template_path = ctx.gui_dir.join("alcom.desktop");
    let template = fs::read_to_string(&template_path)
        .with_context(|| format!("reading {}", template_path.display()))?;

    let categories = match ctx.config.category.as_str() {
        "DeveloperTool" => "Development;",
        "Game" => "Game;",
        "AudioVideo" => "AudioVideo;",
        other => other,
    };

    let result = template
        .replace("{{categories}}", categories)
        .replace("{{comment}}", &ctx.config.short_description)
        .replace("{{exec}}", exec)
        .replace("{{icon}}", "alcom")
        .replace("{{name}}", &ctx.config.product_name)
        // Handle conditional {{#if comment}} block (simplified: always include comment)
        .replace("{{#if comment}}", "")
        .replace("{{/if}}", "");

    Ok(result)
}

pub static LINUX_ICON_RESOLUTIONS: &[&str] =
    &["icons/32x32.png", "icons/64x64.png", "icons/128x128.png"];

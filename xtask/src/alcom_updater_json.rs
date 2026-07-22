use crate::sign_alcom_updater::{secret_key, sign_file};
use anyhow::*;
use base64::Engine;
use chrono::{Timelike, Utc};
use std::path::{Path, PathBuf};
use std::result::Result::Ok;

/// Generates json for tauri updater.
#[derive(clap::Parser)]
pub struct Command {
    #[clap(long = "assets", default_value = "assets")]
    assets_dir: PathBuf,
    #[clap(long = "version")]
    version: String,
    #[clap(long = "sign")]
    sign: bool,
    out_path: PathBuf,
}

impl crate::Command for Command {
    fn run(self) -> Result<i32> {
        create_alcom_updater_json(&self.assets_dir, &self.version, &self.out_path, self.sign)?;
        Ok(0)
    }
}

pub fn create_alcom_updater_json(
    assets_dir: &Path,
    version: &str,
    out_path: &Path,
    sign: bool,
) -> Result<()> {
    // consts
    const DOWNLOAD_URL_BASE: &str =
        "https://github.com/vrc-get/vrc-get/releases/download/gui-v{version}";
    let platform_file_name = [
        ("darwin-x86_64", "ALCOM-{version}-universal.app.tar.gz"),
        ("darwin-aarch64", "ALCOM-{version}-universal.app.tar.gz"),
        ("linux-x86_64", "alcom-{version}-x86_64.AppImage.tar.gz"),
        ("linux-aarch64", "alcom-{version}-aarch64.AppImage.tar.gz"),
        ("windows-x86_64", "ALCOM-{version}-updater.exe"),
        ("windows-aarch64", "ALCOM-{version}-updater.exe"),
    ]
    .into_iter()
    .collect::<Vec<_>>();

    let base_url = DOWNLOAD_URL_BASE.replace("{version}", version);

    // create platforms info
    let mut platforms = serde_json::Map::new();
    for (platform, file_name) in platform_file_name {
        let file_name = file_name.replace("{version}", version);

        std::fs::metadata(assets_dir.join(&file_name)).with_context(|| file_name.clone())?;

        let signature = if sign {
            let private_key = std::env::var("TAURI_SIGNING_PRIVATE_KEY")
                .context("Required environment variable TAURI_SIGNING_PRIVATE_KEY")?;
            let password = std::env::var("TAURI_SIGNING_PRIVATE_KEY_PASSWORD")
                .context("Required environment variable TAURI_SIGNING_PRIVATE_KEY_PASSWORD")?;

            let signature = sign_file(
                &secret_key(&private_key, &password)?,
                &assets_dir.join(&file_name),
            )
            .with_context(|| "failed to sign file")?;

            base64::engine::general_purpose::STANDARD.encode(signature.to_string())
        } else {
            let sig_name = format!("{file_name}.sig");
            std::fs::read_to_string(assets_dir.join(&sig_name)).with_context(|| sig_name.clone())?
        };

        let url = format!("{base_url}/{file_name}");
        platforms.insert(
            platform.to_string(),
            serde_json::json!({
                "signature": signature,
                "url": url,
                "args": [],
            }),
        );
    }

    let args = [
        "/SP-",
        "/SILENT",
        "/NOICONS",
        "!peruser:/CURRENTUSER",
        "!machine:/ALLUSERS",
    ]
    .iter()
    .map(|x| x.to_string())
    .collect::<Vec<_>>();
    platforms["windows-x86_64"]["args"] = args.clone().into();
    platforms["windows-aarch64"]["args"] = args.into();

    let is_beta = version.contains('-');
    let notes = if is_beta {
        // https://github.com/vrc-get/vrc-get/blob/master/CHANGELOG-gui.md#unreleased
        "Please read changelog at https://github.com/vrc-get/vrc-get/blob/master/CHANGELOG-gui.md#unreleased".into()
    } else {
        // https://github.com/vrc-get/vrc-get/blob/master/CHANGELOG-gui.md#101---2025-02-05
        let version = version.replace('.', "");
        let date = Utc::now().format("%Y-%m-%d").to_string();
        format!(
            "Please read changelog at https://github.com/vrc-get/vrc-get/blob/master/CHANGELOG-gui.md#{version}---{date}"
        )
    };

    let updater = serde_json::json!({
        "version": version,
        "notes": notes,
        "pub_date": Utc::now()
            .with_nanosecond(0)
            .unwrap()
            .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
        "platforms": platforms
    });

    let json = serde_json::to_string_pretty(&updater)?;
    std::fs::write(out_path, json).context("write updater.json")?;

    Ok(())
}

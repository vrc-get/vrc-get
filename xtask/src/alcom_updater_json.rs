use anyhow::*;
use chrono::{Timelike, Utc};
use indexmap::IndexMap;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::result::Result::Ok;

/// Generates json for tauri updater.
#[derive(clap::Parser)]
pub struct Command {
    #[clap(long = "assets", default_value = "assets")]
    assets_dir: PathBuf,
    #[clap(long = "version")]
    version: String,
    out_path: PathBuf,
}

impl crate::Command for Command {
    fn run(self) -> Result<i32> {
        create_alcom_updater_json(&self.assets_dir, &self.version, &self.out_path)?;
        Ok(0)
    }
}

#[derive(Serialize)]
struct UpdaterJson<'a> {
    version: &'a str,
    notes: String,
    pub_date: chrono::DateTime<Utc>,
    platforms: IndexMap<String, Platform>,
}

#[derive(serde::Serialize)]
struct Platform {
    signature: String,
    url: String,
}

pub fn create_alcom_updater_json(assets_dir: &Path, version: &str, out_path: &Path) -> Result<()> {
    // consts
    const DOWNLOAD_URL_BASE: &str =
        "https://github.com/vrc-get/vrc-get/releases/download/gui-v{version}";
    let platform_file_name = [
        ("darwin-x86_64", "ALCOM-{version}-universal.app.tar.gz"),
        ("darwin-aarch64", "ALCOM-{version}-universal.app.tar.gz"),
        ("linux-x86_64", "alcom-{version}-x86_64.AppImage.tar.gz"),
        //("linux-aarch64", "alcom-{version}-aarch64.AppImage.tar.gz"),
        ("windows-x86_64", "ALCOM-{version}-x86_64-setup.nsis.zip"),
        //("windows-aarch64", "ALCOM-{version}-aarch64-setup.nsis.zip"),
    ]
    .into_iter()
    .collect::<IndexMap<_, _>>();

    let base_url = DOWNLOAD_URL_BASE.replace("{version}", version);

    // create platforms info
    let mut platforms = IndexMap::new();
    for (platform, file_name) in platform_file_name {
        let file_name = file_name.replace("{version}", version);

        std::fs::metadata(assets_dir.join(&file_name)).with_context(|| file_name.clone())?;

        let sig_name = format!("{file_name}.sig");
        let signature = std::fs::read_to_string(assets_dir.join(&sig_name))
            .with_context(|| sig_name.clone())?;

        let url = format!("{base_url}/{file_name}");
        platforms.insert(platform.to_string(), Platform { signature, url });
    }

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

    let updater = UpdaterJson {
        version,
        notes,
        pub_date: Utc::now().with_nanosecond(0).unwrap(),
        platforms,
    };

    let json = serde_json::to_string_pretty(&updater)?;
    std::fs::write(out_path, json).context("write updater.json")?;

    Ok(())
}

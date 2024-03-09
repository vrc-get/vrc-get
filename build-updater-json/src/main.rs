// see https://tauri.app/v1/guides/distribution/updater/ for json format

use chrono::{Timelike, Utc};
use indexmap::IndexMap;
use serde::Serialize;

#[derive(Serialize)]
struct UpdaterJson {
    version: String,
    notes: String,
    pub_date: chrono::DateTime<Utc>,
    platforms: IndexMap<String, Platform>,
}

#[derive(Serialize)]
struct Platform {
    signature: String,
    url: String,
}

fn main() {
    // consts
    const DOWNLOAD_URL_BASE: &str =
        "https://github.com/vrc-get/vrc-get/releases/download/gui-v{version}";
    let platform_file_name = [
        (
            "darwin-x86_64",
            "vrc-get-gui-{version}-universal.app.tar.gz",
        ),
        (
            "darwin-aarch64",
            "vrc-get-gui-{version}-universal.app.tar.gz",
        ),
        (
            "linux-x86_64",
            "vrc-get-gui-{version}-x86_64.AppImage.tar.gz",
        ),
        /*
        (
            "linux-aarch64",
            "vrc-get-gui-{version}-aarch64.AppImage.tar.gz",
        ),
        */
        (
            "windows-x86_64",
            "vrc-get-gui-{version}-x86_64-setup.nsis.zip",
        ),
        /*
        (
            "windows-aarch64",
            "vrc-get-gui-{version}-aarch64-setup.nsis.zip",
        ),
        */
    ]
    .into_iter()
    .collect::<IndexMap<_, _>>();

    let version = std::env::var("GUI_VERSION").expect("GUI_VERSION not set");

    let base_url = DOWNLOAD_URL_BASE.replace("{version}", &version);

    // create platforms info
    let mut platforms = IndexMap::new();
    for (platform, file_name) in platform_file_name {
        let file_name = file_name.replace("{version}", &version);

        std::fs::metadata(format!("assets/{file_name}"))
            .unwrap_or_else(|e| panic!("{}: {}", file_name, e));

        let signature = std::fs::read_to_string(format!("assets/{file_name}.sig"))
            .unwrap_or_else(|e| panic!("{}.sig: {}", file_name, e));

        let url = format!("{}/{}", base_url, file_name);
        platforms.insert(platform.to_string(), Platform { signature, url });
    }

    let updater = UpdaterJson {
        version,
        notes: "Bug Fixes or New Features".to_string(),
        pub_date: Utc::now().with_nanosecond(0).unwrap(),
        platforms,
    };
    let json = serde_json::to_string_pretty(&updater).unwrap();
    std::fs::write("updater.json", json).expect("write updater.json");
}

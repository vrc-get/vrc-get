use super::os;
use crate::utils::PathBufExt;
use crate::version::UnityVersion;
use either::Either;
use futures::FutureExt;
use futures::future::{join_all, try_join3};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::result;

type Result<T> = result::Result<T, std::io::Error>;

#[derive(Copy, Clone, Debug)]
pub enum ChipArchitecture {
    X86_64,
    ARM64,
}

#[derive(Debug)]
pub struct UnityEditorInHub {
    pub version: UnityVersion,
    pub path: PathBuf,
    pub architecture: Option<ChipArchitecture>,
}

pub async fn load_unity_by_loading_unity_hub_files() -> Result<Vec<UnityEditorInHub>> {
    let install_path = os::app_path()
        .joined("Unity")
        .joined("Hub")
        .joined("Editor");

    let local_settings = LocalSettings::new().await;

    let (a, b, c) = try_join3(
        find_unity_editors_in_folder(&install_path),
        async {
            let Some(install_location) = get_custom_install_location(&local_settings).await else {
                return Ok(Vec::new());
            };
            if install_location == install_path {
                return Ok(Vec::new());
            }
            find_unity_editors_in_folder(&install_location).await
        },
        load_located_editors(&local_settings).map(Ok),
    )
    .await?;

    Ok(a.into_iter()
        .chain(b.into_iter())
        .chain(c.into_iter())
        .collect())
}

async fn get_custom_install_location(local_settings: &LocalSettings) -> Option<PathBuf> {
    let user_setting = local_settings
        .load_setting_file::<String>("secondaryInstallPath.json")
        .await
        .unwrap_or(String::new());
    if !user_setting.is_empty() {
        return Some(PathBuf::from(user_setting));
    }
    let global_setting = &local_settings.machine_wide_install_location;
    if let Some(global_setting) = global_setting {
        if !global_setting.as_os_str().is_empty() {
            return Some(global_setting.clone());
        }
    }
    None
}

async fn find_unity_editors_in_folder(folder_path: &Path) -> Result<Vec<UnityEditorInHub>> {
    let editor_folders = find_unity_editor_folder_in_folder(folder_path).await?;

    Ok(
        join_all(editor_folders.into_iter().map(|folder_path| async move {
            let editor_exe_path = os::editor_path_from_folder(&folder_path);
            let version = os::load_unity_version(&editor_exe_path).await.ok()?;
            let architecture = os::load_editor_architecture(&editor_exe_path).await.ok()?;

            Some(UnityEditorInHub {
                version,
                path: editor_exe_path,
                architecture: Some(architecture),
            })
        }))
        .await
        .into_iter()
        .flatten()
        .collect(),
    )
}

async fn find_unity_editor_folder_in_folder(folder_path: &Path) -> Result<Vec<PathBuf>> {
    fn valid_editor_folder_name(name: &str) -> bool {
        // /^\d+\.\d+\.\d+[abfp]\d+(c\d+)?-?/

        macro_rules! one_or_more {
            ($name: ident, $pat: expr) => {
                let pat = $pat;
                let Some(mut $name) = $name.strip_prefix(pat) else {
                    return false;
                };
                while let Some(name2) = $name.strip_prefix(pat) {
                    $name = name2;
                }
                let $name = $name;
            };
        }

        let is_digit = |c: char| c.is_ascii_digit();

        one_or_more!(name, is_digit);
        let Some(name) = name.strip_prefix('.') else {
            return false;
        };
        one_or_more!(name, is_digit);
        let Some(name) = name.strip_prefix('.') else {
            return false;
        };
        one_or_more!(name, is_digit);
        let Some(name) = name.strip_prefix(['a', 'b', 'f', 'p', 'c', 'x'].as_ref()) else {
            return false;
        };
        one_or_more!(name, is_digit);

        let mut name = name;

        // allow China's f1c1 versions
        if let Some(name2) = name.strip_prefix('c') {
            one_or_more!(name2, is_digit);
            name = name2;
        }

        name.is_empty() || name.starts_with('-')
    }

    match tokio::fs::read_dir(folder_path).await {
        Ok(mut entries) => {
            let mut result = Vec::new();

            while let Some(entry) = entries.next_entry().await? {
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    continue;
                };

                if !entry.file_type().await?.is_dir() {
                    continue;
                }

                if !valid_editor_folder_name(name) {
                    continue;
                }
                if tokio::fs::try_exists(os::editor_path_from_folder(&entry.path()))
                    .await
                    .unwrap_or_default()
                {
                    result.push(entry.path());
                }
            }

            Ok(result)
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

async fn load_located_editors(local_settings: &LocalSettings) -> Vec<UnityEditorInHub> {
    #[derive(Deserialize)]
    struct LocatedEditor {
        #[serde(with = "either::serde_untagged")]
        location: Either<String, Vec<String>>,
        version: String,
        architecture: String,
    }
    #[derive(Deserialize)]
    struct EditorsV2 {
        #[serde(default)]
        data: Vec<LocatedEditor>,
    }

    let Some(editors) = local_settings
        .load_setting_file::<EditorsV2>("editors-v2.json")
        .await
    else {
        return Vec::new();
    };

    let mut result = Vec::new();

    for editor in editors.data {
        let Some(version) = UnityVersion::parse(&editor.version) else {
            continue;
        };
        let architecture = match editor.architecture.as_str() {
            "x86_64" => Some(ChipArchitecture::X86_64),
            "arm64" => Some(ChipArchitecture::ARM64),
            _ => None,
        };
        match editor.location {
            Either::Left(location) => {
                result.push(UnityEditorInHub {
                    version,
                    path: PathBuf::from(location),
                    architecture,
                });
            }
            Either::Right(locations) => {
                for location in locations {
                    result.push(UnityEditorInHub {
                        version,
                        path: PathBuf::from(location),
                        architecture,
                    });
                }
            }
        }
    }

    result
}

fn load_json_or_none<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let data = std::fs::read(path).ok()?;
    serde_json::from_slice(&data).ok()
}

struct LocalSettings {
    user_data_path: PathBuf,
    machine_wide_install_location: Option<PathBuf>,
}

impl LocalSettings {
    pub async fn new() -> Self {
        let user_data_path = os::user_data_path();
        let mut result = Self {
            user_data_path,
            machine_wide_install_location: None,
        };

        #[derive(Deserialize)]
        struct UnityHubSettings {
            #[serde(rename = "machineWideSecondaryInstallLocation")]
            machine_wide_secondary_install_location: Option<PathBuf>,
        }

        macro_rules! load {
            ($expr: expr) => {{
                if let Some(settings) = $expr as Option<UnityHubSettings> {
                    if let Some(machine_wide_secondary_install_location) =
                        settings.machine_wide_secondary_install_location
                    {
                        result.machine_wide_install_location =
                            Some(machine_wide_secondary_install_location);
                    }
                }
            }};
        }

        load!(result.load_setting_file("settings.json").await);
        load!(load_json_or_none(
            &os::global_config_folder().join("services-config.json")
        ));
        load!(result.load_setting_file("Settings").await);

        result
    }

    pub async fn load_setting_file<T: DeserializeOwned>(&self, name: &str) -> Option<T> {
        load_json_or_none(&self.user_data_path.join(name))
    }
}

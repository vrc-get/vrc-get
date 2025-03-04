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

pub async fn find_available_editors() -> Result<Vec<UnityEditorInHub>> {
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

mod os {
    use super::*;
    use crate::utils::PathBufExt;
    use crate::version::UnityVersion;
    use std::path::{Path, PathBuf};

    #[cfg(target_os = "macos")]
    pub use darwin::*;
    #[cfg(target_os = "linux")]
    pub use linux::*;
    #[cfg(target_os = "windows")]
    pub use windows::*;

    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    mod darwin {
        use super::*;
        use tokio::io::AsyncReadExt;

        pub fn app_path() -> PathBuf {
            PathBuf::from("/Applications")
        }

        pub fn user_data_path() -> PathBuf {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .expect("HOME environment variable is not set")
                .joined("Library/Application Support/UnityHub")
        }

        pub fn global_config_folder() -> PathBuf {
            PathBuf::from("/Library/Application Support/Unity/config")
        }

        pub fn editor_path_from_folder(folder: &Path) -> PathBuf {
            folder.join("Unity.app")
        }

        #[cfg(target_os = "macos")] // plist is optional dependency only for macos
        pub async fn load_unity_version(unity: &Path) -> Result<UnityVersion> {
            use std::io::Cursor;

            let plist_path = unity.join("Contents/Info.plist");
            let plist_file = tokio::fs::read(&plist_path).await?;
            let plist =
                plist::Value::from_reader(Cursor::new(plist_file.as_slice())).map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid plist")
                })?;
            plist
                .as_dictionary()
                .and_then(|x| x.get("CFBundleVersion"))
                .and_then(|x| x.as_string())
                .and_then(UnityVersion::parse)
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid version")
                })
        }

        const HEADER_SIZE: usize = 8;

        const fn macho_header(arch: u32) -> [u8; HEADER_SIZE] {
            let mut header = [0u8; 8];
            header[0] = 0xcf;
            header[1] = 0xfa;
            header[2] = 0xed;
            header[3] = 0xfe;
            let arch = &arch.to_le_bytes();
            header[4] = arch[0];
            header[5] = arch[1];
            header[6] = arch[2];
            header[7] = arch[3];
            header
        }

        const MACHO_HEADER_X64: [u8; HEADER_SIZE] = macho_header(7 | 0x0100_0000);
        const MACHO_HEADER_ARM64: [u8; HEADER_SIZE] = macho_header(12u32 | 0x0100_0000);

        pub async fn load_editor_architecture(unity: &Path) -> Result<ChipArchitecture> {
            let exe_path = unity.join("Contents/MacOS/Unity");
            let mut buffer = [0u8; HEADER_SIZE];
            tokio::fs::File::open(exe_path)
                .await?
                .read_exact(&mut buffer)
                .await?;

            if buffer == MACHO_HEADER_X64 {
                Ok(ChipArchitecture::X86_64)
            } else if buffer == MACHO_HEADER_ARM64 {
                Ok(ChipArchitecture::ARM64)
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "unknown architecture",
                ))
            }
        }
    }

    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    mod linux {
        use super::*;

        pub fn app_path() -> PathBuf {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .expect("HOME environment variable is not set")
        }

        pub fn user_data_path() -> PathBuf {
            if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME")
                .take_if(|x| !x.is_empty())
                .map(PathBuf::from)
            {
                config_home.joined("UnityHub")
            } else {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .expect("HOME environment variable is not set")
                    .joined(".config/UnityHub")
            }
        }

        pub fn global_config_folder() -> PathBuf {
            PathBuf::from("/usr/share/unity3d/config")
        }

        pub fn editor_path_from_folder(folder: &Path) -> PathBuf {
            folder.join("Editor/Unity")
        }

        pub async fn load_unity_version(unity: &Path) -> Result<UnityVersion> {
            let version_name = unity
                .parent()
                .and_then(|x| x.parent())
                .and_then(|x| x.file_name())
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid editor location")
                })?;

            let version_name = version_name.to_str().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid version")
            })?;

            let version_name = version_name.trim_start_matches("Unity-");

            UnityVersion::parse(version_name).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid version")
            })
        }

        pub async fn load_editor_architecture(_unity: &Path) -> Result<ChipArchitecture> {
            // Linux ARM is not supported by Unity
            Ok(ChipArchitecture::X86_64)
        }
    }

    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    mod windows {
        use super::*;
        use tokio::io::AsyncReadExt;

        pub fn app_path() -> PathBuf {
            std::env::var_os("PROGRAMFILES")
                .map(PathBuf::from)
                .unwrap_or(PathBuf::from("C:\\Program Files"))
        }

        pub fn user_data_path() -> PathBuf {
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .expect("APPDATA environment variable is not set")
                .joined("UnityHub")
        }

        pub fn global_config_folder() -> PathBuf {
            std::env::var_os("ALLUSERSPROFILE")
                .map(PathBuf::from)
                .expect("ALLUSERSPROFILE environment variable is not set")
                .joined("Unity\\config")
        }

        pub fn editor_path_from_folder(folder: &Path) -> PathBuf {
            folder.join("Editor\\Unity.exe")
        }

        #[cfg(target_os = "windows")] // windows-rs is optional dependency only for macos
        #[allow(unsafe_code)]
        pub async fn load_unity_version(unity: &Path) -> Result<UnityVersion> {
            use ::windows::Win32::Storage::FileSystem::*;
            use ::windows::core::HSTRING;

            // TODO: make fully async
            unsafe {
                let filename = HSTRING::from(unity);
                let size = GetFileVersionInfoSizeW(&filename, None);

                let mut version_info = vec![0u8; size as usize];

                GetFileVersionInfoW(&filename, None, size, version_info.as_mut_ptr() as _)?;

                let mut buffer_ptr = std::ptr::null::<u16>();
                let mut size: u32 = 0;

                if !VerQueryValueW(
                    version_info.as_ptr() as _,
                    &HSTRING::from(r"\StringFileInfo\040904b0\Unity Version"),
                    &mut buffer_ptr as *mut _ as _,
                    &mut size as *mut _,
                )
                .as_bool()
                {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "version info not found",
                    ));
                }

                let slice = std::slice::from_raw_parts(buffer_ptr, size as usize);
                let str = String::from_utf16_lossy(slice);
                let version = str.split_once('_').unwrap_or((&str, "")).0;

                UnityVersion::parse(version).ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid version")
                })
            }
        }

        pub async fn load_editor_architecture(unity: &Path) -> Result<ChipArchitecture> {
            let mut buffer = [0u8; 512];

            tokio::fs::File::open(unity)
                .await?
                .read_exact(&mut buffer)
                .await?;

            let coff_offset = u32::from_le_bytes(buffer[0x3c..][..4].try_into().unwrap());
            let coff_header_size = 24;
            if coff_offset + coff_header_size >= buffer.len() as u32 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid PE header or too big offset",
                ));
            }

            let coff_header = &buffer[coff_offset as usize..][..(coff_header_size as usize)];
            if &coff_header[0..4] != b"PE\0\0" {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid PE header",
                ));
            }

            let machine = u16::from_le_bytes(coff_header[4..][..2].try_into().unwrap());

            match machine {
                0x8664 => Ok(ChipArchitecture::X86_64),
                0xaa64 => Ok(ChipArchitecture::ARM64),
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "unknown architecture",
                )),
            }
        }
    }
}

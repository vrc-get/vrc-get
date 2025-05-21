use crate::unity_hub::find_unity_from_unity_hub_logic::*;
use crate::utils::PathBufExt;
use crate::version::UnityVersion;
use std::path::{Path, PathBuf};
use std::result;

#[cfg(target_os = "macos")]
pub use darwin::*;
#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "windows")]
pub use windows::*;

type Result<T> = result::Result<T, std::io::Error>;

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
        let plist = plist::Value::from_reader(Cursor::new(plist_file.as_slice()))
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid plist"))?;
        plist
            .as_dictionary()
            .and_then(|x| x.get("CFBundleVersion"))
            .and_then(|x| x.as_string())
            .and_then(UnityVersion::parse)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid version"))
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

        UnityVersion::parse(version_name)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid version"))
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
        use std::io;

        fn inner(unity: &Path) -> Result<UnityVersion> {
            use ::windows::Win32::Storage::FileSystem::*;
            use ::windows::core::HSTRING;
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
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "version info not found",
                    ));
                }

                let slice = std::slice::from_raw_parts(buffer_ptr, size as usize);
                let str = String::from_utf16_lossy(slice);
                let version = str.split_once('_').unwrap_or((&str, "")).0;

                UnityVersion::parse(version)
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid version"))
            }
        }

        let unity = unity.to_path_buf();
        match tokio::task::spawn_blocking(move || inner(&unity)).await {
            Ok(result) => result,
            Err(_) => Err(io::Error::other("background task failed")),
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

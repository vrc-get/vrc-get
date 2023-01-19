//! This module contains vpm core implementation
//!
//! This module might be a separated crate.

use std::collections::HashMap;
use std::env::var;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

pub mod structs;

/// This struct holds global state (will be saved on %LOCALAPPDATA% of VPM.
#[derive(Debug)]
pub struct Environment {
    /// config folder.
    /// On windows, `%APPDATA%\\VRChatCreatorCompanion`.
    /// On posix, `${XDG_DATA_HOME}/VRChatCreatorCompanion`.
    global_dir: PathBuf,
    /// parsed settings
    settings: structs::setting::SettingsJson,

    /// Cache
    cached_repos: HashMap<String, structs::repository::LocalCachedRepository>,
}

impl Environment {
    pub fn load_default() -> io::Result<Environment> {
        let mut folder = Environment::get_local_config_folder();
        folder.push("VRChatCreatorCompanion");
        let folder = folder;

        Ok(Environment {
            settings: load_json_optional(&folder.join("settings.json"))?,
            global_dir: folder,
            cached_repos: HashMap::new(),
        })
    }

    #[cfg(windows)]
    fn get_local_config_folder() -> PathBuf {
        // use CLSID?
        if let Some(local_appdata) = env::var_os("CSIDL_LOCAL_APPDATA") {
            return local_appdata.into();
        }
        // fallback: use HOME
        if let Some(home_folder) = env::var_os("HOMEPATH") {
            let mut path = PathBuf::from(home_folder);
            path.push("AppData\\Local");
            return path;
        }

        panic!("no CSIDL_LOCAL_APPDATA nor HOMEPATH are set!")
    }

    #[cfg(not(windows))]
    fn get_local_config_folder() -> PathBuf {
        if let Some(data_home) = env::var_os("XDG_DATA_HOME") {
            return data_home.into();
        }

        // fallback: use HOME
        if let Some(home_folder) = env::var_os("HOME") {
            let mut path = PathBuf::from(home_folder);
            path.push(".local/share");
            return path;
        }

        panic!("no XDG_DATA_HOME nor HOME are set!")
    }
}

#[derive(Debug)]
pub struct UnityProject {
    /// path to `Packages` folder.
    packages_dir: PathBuf,
    /// manifest.json
    manifest: structs::manifest::VpmManifest,
}

impl UnityProject {
    pub fn find_unity_project(unity_project: Option<PathBuf>) -> io::Result<UnityProject> {
        let mut unity_found = unity_project
            .ok_or(())
            .or_else(|_| UnityProject::find_unity_project_path())?;
        unity_found.push("Packages");

        let mut manifest = unity_found.join("vpm-manifest.json");

        Ok(UnityProject {
            packages_dir: unity_found,
            manifest: load_json_optional(&manifest)?,
        })
    }

    fn find_unity_project_path() -> io::Result<PathBuf> {
        let mut candidate = env::current_dir()?;

        loop {
            candidate.push("Packages");
            candidate.push("vpm-manifest.json");

            if candidate.exists() {
                // if there's vpm-manifest.json, it's project path
                candidate.pop();
                candidate.pop();
                return Ok(candidate);
            }

            // replace vpm-manifest.json -> manifest.json
            candidate.pop();
            candidate.push("manifest.json");

            if candidate.exists() {
                // if there's manifest.json (which is manifest.json), it's project path
                candidate.pop();
                candidate.pop();
                return Ok(candidate);
            }

            // remove Packages/manifest.json
            candidate.pop();
            candidate.pop();

            // go to parent dir
            if !candidate.pop() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Unity project Not Found",
                ));
            }
        }
    }
}

fn load_json_optional<T>(manifest_path: &Path) -> io::Result<T>
where
    T: serde::de::DeserializeOwned + Default,
{
    match fs::File::open(manifest_path) {
        Ok(file) => Ok(serde_json::from_reader::<_, T>(file)?),
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(Default::default()),
        Err(e) => Err(e),
    }
}

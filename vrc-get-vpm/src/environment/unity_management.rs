use crate::io::EnvironmentIo;
use crate::utils::{check_absolute_path, normalize_path};
use crate::version::UnityVersion;
use crate::{io, Environment, HttpClient};
use bson::oid::ObjectId;
use log::info;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(crate) static COLLECTION: &str = "unityVersions";

impl<T: HttpClient, IO: EnvironmentIo> Environment<T, IO> {
    pub fn get_unity_installations(&self) -> io::Result<Vec<UnityInstallation>> {
        Ok(self.get_db()?.get_values(COLLECTION)?)
    }

    pub async fn add_unity_installation(
        &mut self,
        path: &str,
        version: UnityVersion,
    ) -> io::Result<()> {
        check_absolute_path(path)?;
        self.add_unity_installation_internal(path, version, false)
            .await
    }

    async fn add_unity_installation_internal(
        &mut self,
        path: &str,
        version: UnityVersion,
        is_from_hub: bool,
    ) -> io::Result<()> {
        let db = self.get_db()?;

        let mut installation = UnityInstallation::new(path.into(), Some(version), false);

        installation.loaded_from_hub = is_from_hub;

        db.insert(COLLECTION, &installation)?;

        Ok(())
    }

    pub async fn remove_unity_installation(&mut self, unity: &UnityInstallation) -> io::Result<()> {
        self.get_db()?.delete(COLLECTION, unity.id)?;

        Ok(())
    }

    pub fn find_most_suitable_unity(
        &self,
        expected: UnityVersion,
    ) -> io::Result<Option<UnityInstallation>> {
        let mut revision_match = None;
        let mut minor_match = None;
        let mut major_match = None;

        for unity in self.get_unity_installations()? {
            if let Some(version) = unity.version() {
                if version == expected {
                    return Ok(Some(unity));
                }

                if version.major() == expected.major() {
                    if version.minor() == expected.minor() {
                        if version.revision() == expected.revision() {
                            revision_match = Some(unity);
                        } else {
                            minor_match = Some(unity);
                        }
                    } else {
                        major_match = Some(unity);
                    }
                } else {
                    continue;
                }
            }
        }

        Ok(revision_match.or(minor_match).or(major_match))
    }
}

/// UnityHub Operations
impl<T: HttpClient, IO: EnvironmentIo> Environment<T, IO> {
    fn default_unity_hub_path() -> &'static [&'static str] {
        // https://docs.unity3d.com/hub/manual/HubCLI.html
        #[cfg(windows)]
        {
            lazy_static::lazy_static! {
                static ref INSTALLATIONS: &'static [&'static str] = {
                    // https://github.com/vrc-get/vrc-get/issues/579
                    if let Some(unity_hub_from_regi) =
                        winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
                            .open_subkey(r"Software\Unity Technologies\Hub")
                            .ok()
                            .and_then(|key| key.get_value("InstallPath").ok())
                            .and_then(|str: std::ffi::OsString| str.into_string().ok())
                            .map(|s| PathBuf::from(s))
                            .map(|mut p| {
                                p.push("Unity Hub.exe");
                                p
                            })
                            .map(|p| p.into_os_string().into_string().unwrap()) {
                        vec![unity_hub_from_regi.leak(), "C:\\Program Files\\Unity Hub\\Unity Hub.exe"].leak()
                    } else {
                        &["C:\\Program Files\\Unity Hub\\Unity Hub.exe"]
                    }
                };
            }

            INSTALLATIONS.as_ref()
        }
        #[cfg(target_os = "macos")]
        {
            &["/Applications/Unity Hub.app/Contents/MacOS/Unity Hub"]
        }
        #[cfg(target_os = "linux")]
        {
            // for linux,
            lazy_static::lazy_static! {
                static ref USER_INSTALLATION: String = {
                    let home = std::env::var("HOME").expect("HOME not set");
                    format!("{}/Applications/Unity Hub.AppImage", home)
                };
                static ref INSTALLATIONS: [&'static str; 3] =
                    [&USER_INSTALLATION, "/usr/bin/unity-hub", "/opt/unityhub/unityhub"];
            }

            INSTALLATIONS.as_ref()
        }
    }

    pub async fn find_unity_hub(&mut self) -> io::Result<Option<String>> {
        let path = self.settings.unity_hub_path();
        if !path.is_empty() && self.io.is_file(path.as_ref()).await {
            // if configured one is valid path to file, return it
            return Ok(Some(path.to_string()));
        }

        // if not, try default paths

        for &path in Self::default_unity_hub_path() {
            if self.io.is_file(path.as_ref()).await {
                self.settings.set_unity_hub_path(path);
                return Ok(Some(path.to_string()));
            }
        }

        Ok(None)
    }

    pub async fn update_unity_from_unity_hub_and_fs(
        &mut self,
        path_and_version_from_hub: &[(UnityVersion, PathBuf)],
    ) -> io::Result<()> {
        let paths_from_hub = path_and_version_from_hub
            .iter()
            .map(|(_, path)| path.as_path())
            .collect::<HashSet<_>>();

        let db = self.get_db()?;

        let mut installed = HashSet::new();

        for mut in_db in db.get_values::<UnityInstallation>(COLLECTION)? {
            let path = Path::new(in_db.path());
            if !self.io.is_file(path).await {
                // if the unity editor not found, remove it from the db
                info!("Removed Unity that is not exists: {}", in_db.path());
                db.delete(COLLECTION, in_db.id)?;
                continue;
            }

            if installed.contains(path) {
                // if the unity editor is already installed, remove it from the db
                info!("Removed duplicated Unity: {}", in_db.path());
                db.delete(COLLECTION, in_db.id)?;
                continue;
            }

            installed.insert(PathBuf::from(path));

            let normalized = normalize_path(path).into_os_string().into_string().unwrap();
            let exists_in_hub = paths_from_hub.contains(path);

            let mut update = false;

            if exists_in_hub != in_db.loaded_from_hub() {
                in_db.loaded_from_hub = exists_in_hub;
                update = true;
            }

            if normalized != in_db.path() {
                in_db.path = normalized.into();
                update = true;
            }

            if update {
                db.update(COLLECTION, &in_db)?;
            }
        }

        for &(version, ref path) in path_and_version_from_hub {
            if !installed.contains(path) {
                if version < UnityVersion::new_f1(2019, 4, 0) {
                    info!(
                        "Ignoring Unity from Unity Hub since old: {}",
                        path.display()
                    );
                    continue;
                }
                info!("Adding Unity from Unity Hub: {}", path.display());
                self.add_unity_installation_internal(&path.to_string_lossy(), version, true)
                    .await?;
            }
        }

        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
pub struct UnityInstallation {
    #[serde(rename = "_id")]
    id: ObjectId,
    #[serde(rename = "Path")]
    path: Box<str>,
    #[serde(rename = "Version")]
    #[serde(deserialize_with = "default_if_err")]
    version: Option<UnityVersion>,
    #[serde(rename = "LoadedFromHub")]
    loaded_from_hub: bool,
}

impl UnityInstallation {
    fn new(path: Box<str>, version: Option<UnityVersion>, loaded_from_hub: bool) -> Self {
        Self {
            id: ObjectId::new(),
            path,
            version,
            loaded_from_hub,
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn version(&self) -> Option<UnityVersion> {
        self.version
    }

    pub fn loaded_from_hub(&self) -> bool {
        self.loaded_from_hub
    }
}

// for unity 2018.x or older, VCC will parse version as "2018.4.0" instead of "2018.4.0f1"
// and 2018.4.31f1 as "2018.4" instead of "2018.4.31f1"
// Therefore, we need skip parsing such a version string.
fn default_if_err<'de, D, T>(de: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    match T::deserialize(de) {
        Ok(v) => Ok(v),
        Err(_) => Ok(T::default()),
    }
}

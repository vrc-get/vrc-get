use crate::io::EnvironmentIo;
use crate::version::UnityVersion;
use crate::{io, Environment, HttpClient};
use log::info;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use vrc_get_litedb::UnityVersion as DbUnityVersion;

impl<T: HttpClient, IO: EnvironmentIo> Environment<T, IO> {
    pub fn get_unity_installations(&self) -> io::Result<Vec<UnityInstallation>> {
        Ok(self
            .get_db()?
            .get_unity_versions()?
            .into_vec()
            .into_iter()
            .map(UnityInstallation::new)
            .collect())
    }

    pub async fn add_unity_installation(
        &mut self,
        path: &str,
        version: UnityVersion,
    ) -> io::Result<()> {
        let db = self.get_db()?;

        let installation =
            DbUnityVersion::new(path.into(), version.to_string().into_boxed_str(), false);

        db.insert_unity_version(&installation)?;

        Ok(())
    }

    pub async fn remove_unity_installation(&mut self, unity: &UnityInstallation) -> io::Result<()> {
        self.get_db()?.delete_unity_version(unity.inner.id())?;

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
        let path = self.settings.unity_hub();
        if !path.is_empty() && self.io.is_file(path.as_ref()).await {
            // if configured one is valid path to file, return it
            return Ok(Some(path.to_string()));
        }

        // if not, try default paths

        for &path in Self::default_unity_hub_path() {
            if self.io.is_file(path.as_ref()).await {
                self.settings.set_unity_hub(path);
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

        for mut in_db in db.get_unity_versions()?.into_vec() {
            if !self.io.is_file(in_db.path().as_ref()).await {
                // if the unity editor not found, remove it from the db
                info!("Removed Unity that is not exists: {}", in_db.path());
                db.delete_unity_version(in_db.id())?;
                continue;
            }

            installed.insert(PathBuf::from(in_db.path()));

            let exists_in_hub = paths_from_hub.contains(Path::new(in_db.path()));

            if exists_in_hub != in_db.loaded_from_hub() {
                in_db.set_loaded_from_hub(exists_in_hub);
                db.update_unity_version(&in_db)?;
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
                self.add_unity_installation(&path.to_string_lossy(), version)
                    .await?;
            }
        }

        Ok(())
    }
}

#[allow(dead_code)]
pub struct UnityInstallation {
    inner: DbUnityVersion,
}

impl UnityInstallation {
    pub(crate) fn new(inner: DbUnityVersion) -> Self {
        Self { inner }
    }

    pub fn path(&self) -> &str {
        self.inner.path()
    }

    pub fn version(&self) -> Option<UnityVersion> {
        self.inner.version().and_then(UnityVersion::parse)
    }

    pub fn loaded_from_hub(&self) -> bool {
        self.inner.loaded_from_hub()
    }
}

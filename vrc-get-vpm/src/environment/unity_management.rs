use crate::environment::{Settings, VccDatabaseConnection};
use crate::io;
use crate::io::EnvironmentIo;
use crate::unity_hub::get_executable_path;
use crate::utils::{check_absolute_path, normalize_path};
use crate::version::UnityVersion;
use futures::TryStreamExt;
use log::info;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::pin::pin;
use vrc_get_litedb::bson::Document;
use vrc_get_litedb::document;
use vrc_get_litedb::engine::{BsonAutoId, TransactionLiteEngine};

pub(crate) static COLLECTION: &str = "unityVersions";
static PATH: &str = "Path";
static VERSION: &str = "Version";
static LOADED_FROM_HUB: &str = "LoadedFromHub";

impl VccDatabaseConnection {
    pub async fn get_unity_installations(&self) -> io::Result<Vec<UnityInstallation>> {
        Ok(self
            .db
            .get_all(COLLECTION)
            .map_ok(UnityInstallation::from_document)
            .try_collect()
            .await?)
    }

    pub async fn add_unity_installation(
        &mut self,
        path: &str,
        version: UnityVersion,
    ) -> io::Result<()> {
        check_absolute_path(path)?;
        Ok(self
            .db
            .with_transaction(async |db| {
                Ok(Self::add_unity_installation_internal(db, path, version, false).await?)
            })
            .await?)
    }

    async fn add_unity_installation_internal(
        db: &mut TransactionLiteEngine<'_>,
        path: &str,
        version: UnityVersion,
        is_from_hub: bool,
    ) -> io::Result<()> {
        let installation = UnityInstallation::new(path.into(), Some(version), is_from_hub);

        db.insert(COLLECTION, vec![installation.bson], BsonAutoId::ObjectId)
            .await?;

        Ok(())
    }

    pub async fn remove_unity_installation(&mut self, unity: &UnityInstallation) -> io::Result<()> {
        self.db
            .delete(COLLECTION, &[unity.bson["_id"].clone()])
            .await?;

        Ok(())
    }

    pub async fn find_most_suitable_unity(
        &self,
        expected: UnityVersion,
    ) -> io::Result<Option<UnityInstallation>> {
        let mut revision_match = None;
        let mut minor_match = None;
        let mut major_match = None;

        let mut stream = pin!(self.db.get_all(COLLECTION));
        while let Some(unity) = stream.try_next().await? {
            let unity = UnityInstallation::from_document(unity);
            if unity.path().is_none() {
                continue;
            }
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

    pub async fn update_unity_from_unity_hub_and_fs(
        &mut self,
        path_and_version_from_hub: &[(UnityVersion, PathBuf)],
        io: &impl EnvironmentIo,
    ) -> io::Result<()> {
        let path_and_version_from_hub = path_and_version_from_hub
            .iter()
            .map(|(version, path)| (version, get_executable_path(path)))
            .collect::<Vec<_>>();
        let paths_from_hub = path_and_version_from_hub
            .iter()
            .map(|(_, path)| path.as_ref())
            .collect::<HashSet<_>>();

        self.db
            .with_transaction(async |db: &mut TransactionLiteEngine| {
                let mut update = Vec::new();
                let mut delete = Vec::new();

                let mut registered = HashSet::new();

                let mut stream = pin!(self.db.get_all(COLLECTION));
                while let Some(mut in_db) = stream.try_next().await? {
                    let Some(path) = in_db[PATH].as_str() else {
                        // if the unity editor not found, remove it from the db
                        info!("Removed Unity has no path: {:?}", in_db["_id"]);
                        delete.push(in_db["_id"].clone());
                        continue;
                    };

                    let path_path = Path::new(path);
                    if !io.is_file(path_path).await {
                        // if the unity editor not found, remove it from the db
                        info!("Removed Unity that is not exists: {}", path);
                        delete.push(in_db["_id"].clone());
                        continue;
                    }

                    if registered.contains(path) {
                        // if the unity editor is already installed, remove it from the db
                        info!("Removed duplicated Unity: {}", path);
                        delete.push(in_db["_id"].clone());
                        continue;
                    }

                    registered.insert(path.to_string());

                    let normalized = normalize_path(path.as_ref())
                        .into_os_string()
                        .into_string()
                        .unwrap();
                    let exists_in_hub = paths_from_hub.contains(Path::new(path));

                    let mut changed = false;

                    if normalized != path {
                        in_db.insert(PATH, normalized);
                        changed = true;
                    }

                    if Some(exists_in_hub) != in_db[LOADED_FROM_HUB].as_bool() {
                        in_db.insert(LOADED_FROM_HUB, exists_in_hub);
                        changed = true;
                    }

                    if changed {
                        update.push(in_db);
                    }
                }

                self.db.delete(COLLECTION, &delete).await?;
                self.db.update(COLLECTION, update).await?;

                for &(&version, ref path) in &path_and_version_from_hub {
                    let Some(path) = path.as_os_str().to_str() else {
                        info!(
                            "Ignoring Unity from Unity Hub since non-utf8 path: {}",
                            path.display()
                        );
                        continue;
                    };
                    if !registered.contains(path) {
                        if version < UnityVersion::new_f1(2019, 4, 0) {
                            info!("Ignoring Unity from Unity Hub since old: {}", path);
                            continue;
                        }
                        info!("Adding Unity from Unity Hub: {}", path);
                        Self::add_unity_installation_internal(db, path, version, true).await?;
                    }
                }

                Ok(())
            })
            .await?;

        Ok(())
    }
}

pub async fn find_unity_hub(
    settings: &mut Settings,
    io: &impl EnvironmentIo,
) -> io::Result<Option<String>> {
    let path = settings.unity_hub_path();
    if !path.is_empty() && io.is_file(path.as_ref()).await {
        // if configured one is valid path to file, return it
        return Ok(Some(path.to_string()));
    }

    // if not, try default paths

    for &path in default_unity_hub_path() {
        if io.is_file(path.as_ref()).await {
            settings.set_unity_hub_path(path);
            return Ok(Some(path.to_string()));
        }
    }

    Ok(None)
}

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
                        .and_then(|key| key.get_value("InstallLocation").ok())
                        .and_then(|str: std::ffi::OsString| str.into_string().ok())
                        .map(PathBuf::from)
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
            static ref INSTALLATIONS: [&'static str; 4] =
                [
                    &USER_INSTALLATION,
                    "/usr/bin/unity-hub",
                    // apt package
                    "/opt/unityhub/unityhub",
                    // flatpak
                    "/var/lib/flatpak/export/bin/com.unity.UnityHub",
                ];
        }

        INSTALLATIONS.as_ref()
    }
}

pub struct UnityInstallation {
    bson: Document,
}

impl UnityInstallation {
    fn new(path: Box<str>, version: Option<UnityVersion>, loaded_from_hub: bool) -> Self {
        Self {
            bson: document! {
                PATH => path.as_ref(),
                VERSION => version.as_ref().map(ToString::to_string),
                LOADED_FROM_HUB => loaded_from_hub,
            },
        }
    }

    fn from_document(bson: Document) -> Self {
        Self { bson }
    }

    pub fn path(&self) -> Option<&str> {
        self.bson[PATH].as_str()
    }

    pub fn version(&self) -> Option<UnityVersion> {
        self.bson[VERSION].as_str().and_then(UnityVersion::parse)
    }

    pub fn loaded_from_hub(&self) -> bool {
        self.bson[LOADED_FROM_HUB].as_bool().unwrap_or(false)
    }
}

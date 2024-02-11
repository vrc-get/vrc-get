use crate::io::EnvironmentIo;
use crate::version::UnityVersion;
use crate::{io, Environment, HttpClient};
use vrc_get_litedb::UnityVersion as DbUnityVersion;

impl<T: HttpClient, IO: EnvironmentIo> Environment<T, IO> {
    pub fn get_unity_installations(&mut self) -> io::Result<Vec<UnityInstallation>> {
        Ok(self
            .get_db()?
            .get_unity_versions()?
            .into_vec()
            .into_iter()
            .map(UnityInstallation::new)
            .collect())
    }

    pub async fn add_unity_installation(&mut self, path: &str) -> io::Result<UnityVersion> {
        let output = self
            .io
            .command_output(path.as_ref(), &["-version".as_ref()])
            .await?;

        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid unity installation at {}", path),
            ));
        }

        let stdout = &output.stdout[..];
        let index = stdout
            .iter()
            .position(|&x| x == b' ')
            .unwrap_or(stdout.len());

        let version = std::str::from_utf8(&stdout[..index])
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid version"))?;

        let version = UnityVersion::parse(version)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid version"))?;

        let installation =
            DbUnityVersion::new(path.into(), version.to_string().into_boxed_str(), false);

        self.get_db()?.insert_unity_version(&installation)?;

        Ok(version)
    }

    pub async fn remove_unity_installation(&mut self, unity: &UnityInstallation) -> io::Result<()> {
        self.get_db()?.delete_unity_version(unity.inner.id())?;

        Ok(())
    }
}

#[allow(dead_code)]
pub struct UnityInstallation {
    inner: DbUnityVersion,
}

impl UnityInstallation {
    pub fn new(inner: DbUnityVersion) -> Self {
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

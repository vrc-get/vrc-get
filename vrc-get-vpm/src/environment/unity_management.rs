use crate::io::EnvironmentIo;
use crate::version::UnityVersion;
use crate::{io, Environment, HttpClient};

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
}

#[allow(dead_code)]
pub struct UnityInstallation {
    inner: vrc_get_litedb::UnityVersion,
}

impl UnityInstallation {
    pub fn new(inner: vrc_get_litedb::UnityVersion) -> Self {
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

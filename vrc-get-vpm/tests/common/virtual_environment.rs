use crate::common::{PackageCollection, VirtualFileSystem};
use indexmap::IndexMap;
use serde_json::json;
use vrc_get_vpm::io::{EnvironmentIo, IoTrait};
use vrc_get_vpm::unity_project::pending_project_changes::Remove;
use vrc_get_vpm::version::{Version, VersionRange};
use vrc_get_vpm::{EnvironmentIoHolder, PackageManifest, RemotePackageDownloader, UnityProject};

pub struct VirtualEnvironment {
    vfs: VirtualFileSystem,
}

impl VirtualEnvironment {
    pub fn new(vfs: VirtualFileSystem) -> Self {
        Self { vfs }
    }
}

impl EnvironmentIoHolder for VirtualEnvironment {
    type EnvironmentIo = VirtualFileSystem;
    fn io(&self) -> &Self::EnvironmentIo {
        &self.vfs
    }
}

impl RemotePackageDownloader for VirtualEnvironment {
    type FileStream = futures::io::Cursor<&'static [u8]>;

    fn get_package(
        &self,
        _repository: &vrc_get_vpm::repository::LocalCachedRepository,
        _package: &PackageManifest,
    ) -> impl futures::Future<Output = std::io::Result<Self::FileStream>> + Send {
        std::future::ready(Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Remove Access",
        )))
    }
}

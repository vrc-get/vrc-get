use crate::common::{PackageCollection, VirtualFileSystem};
use indexmap::IndexMap;
use serde_json::json;
use std::future::Future;
use std::io;
use std::path::PathBuf;
use vrc_get_vpm::io::{EnvironmentIo, IoTrait, ProjectIo};
use vrc_get_vpm::unity_project::pending_project_changes::Remove;
use vrc_get_vpm::version::{Version, VersionRange};
use vrc_get_vpm::{
    Environment, EnvironmentIoHolder, HttpClient, PackageInfo, PackageInstaller, PackageManifest,
    UnityProject,
};

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

impl PackageInstaller for VirtualEnvironment {
    fn install_package(
        &self,
        _: &impl ProjectIo,
        _: PackageInfo<'_>,
    ) -> impl Future<Output = std::io::Result<()>> {
        std::future::ready(Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "install_package not supported in VirtualEnvironment",
        )))
    }
}

use crate::io;
use crate::io::ProjectIo;
use crate::unity_project::LockedDependencyInfo;
use crate::utils::{load_json_or_default, SaveController};
use crate::version::{DependencyRange, Version, VersionRange};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

const MANIFEST_PATH: &str = "Packages/vpm-manifest.json";

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AsJson {
    #[serde(default)]
    dependencies: IndexMap<Box<str>, VpmDependency>,
    #[serde(default)]
    locked: IndexMap<Box<str>, VpmLockedDependency>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VpmDependency {
    pub version: DependencyRange,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VpmLockedDependency {
    pub version: Version,
    #[serde(default, skip_serializing_if = "indexmap::IndexMap::is_empty")]
    pub dependencies: IndexMap<Box<str>, VersionRange>,
}

#[derive(Debug)]
pub(super) struct VpmManifest {
    controller: SaveController<AsJson>,
}

impl VpmManifest {
    pub(super) async fn load(io: &impl ProjectIo) -> io::Result<Self> {
        Ok(Self {
            controller: SaveController::new(
                load_json_or_default(io, MANIFEST_PATH.as_ref()).await?,
            ),
        })
    }

    pub(super) fn dependencies(&self) -> impl Iterator<Item = (&str, &DependencyRange)> {
        self.controller
            .dependencies
            .iter()
            .map(|(name, dep)| (name.as_ref(), &dep.version))
    }

    pub(super) fn get_dependency(&self, package: &str) -> Option<&DependencyRange> {
        self.controller
            .dependencies
            .get(package)
            .map(|x| &x.version)
    }

    pub(super) fn all_locked(&self) -> impl Iterator<Item = LockedDependencyInfo> {
        self.controller.locked.iter().map(|(name, dep)| {
            LockedDependencyInfo::new(name.as_ref(), &dep.version, &dep.dependencies)
        })
    }

    pub(super) fn get_locked(&self, package: &str) -> Option<LockedDependencyInfo> {
        self.controller
            .locked
            .get_key_value(package)
            .map(|(package, x)| LockedDependencyInfo::new(package, &x.version, &x.dependencies))
    }

    pub(super) fn add_dependency(&mut self, name: &str, version: DependencyRange) {
        self.controller
            .as_mut()
            .dependencies
            .insert(name.into(), VpmDependency { version });
    }

    pub(super) fn add_locked(
        &mut self,
        name: &str,
        version: Version,
        dependencies: IndexMap<Box<str>, VersionRange>,
    ) {
        self.controller.as_mut().locked.insert(
            name.into(),
            VpmLockedDependency {
                version,
                dependencies,
            },
        );
    }

    pub(crate) fn remove_packages<'a>(&mut self, names: impl Iterator<Item = &'a str>) {
        for name in names {
            self.controller.as_mut().locked.shift_remove(name);
            self.controller.as_mut().dependencies.shift_remove(name);
        }
    }

    pub(super) async fn save(&mut self, io: &impl ProjectIo) -> io::Result<()> {
        self.controller.save(io, MANIFEST_PATH.as_ref()).await
    }
}

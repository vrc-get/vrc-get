use crate::io;
use crate::io::DefaultProjectIo;
use crate::unity_project::LockedDependencyInfo;
use crate::utils::{SaveController, load_json_or_default, save_json};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<IndexMap<Box<str>, VersionRange>>,
}

#[derive(Debug)]
pub(super) struct VpmManifest {
    controller: SaveController<AsJson>,
}

impl VpmManifest {
    pub(super) async fn load(io: &DefaultProjectIo) -> io::Result<Self> {
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

    pub(super) fn all_locked(&self) -> impl Iterator<Item = LockedDependencyInfo<'_>> {
        self.controller.locked.iter().map(|(name, dep)| {
            LockedDependencyInfo::new(name.as_ref(), &dep.version, dep.dependencies.as_ref())
        })
    }

    pub(super) fn get_locked(&self, package: &str) -> Option<LockedDependencyInfo<'_>> {
        self.controller
            .locked
            .get_key_value(package)
            .map(|(package, x)| {
                LockedDependencyInfo::new(package, &x.version, x.dependencies.as_ref())
            })
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
                dependencies: Some(dependencies),
            },
        );
    }

    pub(crate) fn remove_packages<'a>(&mut self, names: impl Iterator<Item = &'a str>) {
        for name in names {
            self.controller.as_mut().locked.shift_remove(name);
            self.controller.as_mut().dependencies.shift_remove(name);
        }
    }

    pub(crate) fn has_any(&self) -> bool {
        !self.controller.locked.is_empty() || !self.controller.dependencies.is_empty()
    }

    pub(super) async fn save(&mut self, io: &DefaultProjectIo) -> io::Result<()> {
        self.controller
            .save(|json| save_json(io, MANIFEST_PATH.as_ref(), json))
            .await
    }

    pub(super) fn to_json(&self) -> io::Result<Vec<u8>> {
        crate::utils::to_vec_pretty_os_eol(&*self.controller)
    }
}

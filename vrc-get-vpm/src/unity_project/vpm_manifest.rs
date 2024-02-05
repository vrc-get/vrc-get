use crate::utils::to_vec_pretty_os_eol;
use crate::version::DependencyRange;
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AsJson {
    #[serde(default)]
    dependencies: IndexMap<String, VpmDependency>,
    #[serde(default)]
    locked: IndexMap<String, VpmLockedDependency>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VpmDependency {
    pub version: DependencyRange,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VpmLockedDependency {
    pub version: Version,
    #[serde(default, skip_serializing_if = "indexmap::IndexMap::is_empty")]
    pub dependencies: IndexMap<String, VersionRange>,
}

#[derive(Debug)]
pub(super) struct VpmManifest {
    as_json: AsJson,
    changed: bool,
}

impl VpmManifest {
    pub(super) async fn from(manifest: &Path) -> io::Result<Self> {
        Ok(Self {
            as_json: load_json_or_default(manifest).await?,
            changed: false,
        })
    }

    pub(super) fn dependencies(&self) -> impl Iterator<Item = (&str, &DependencyRange)> {
        self.as_json
            .dependencies
            .iter()
            .map(|(name, dep)| (name.as_str(), &dep.version))
    }

    pub(super) fn get_dependency(&self, package: &str) -> Option<&DependencyRange> {
        self.as_json.dependencies.get(package).map(|x| &x.version)
    }

    pub(super) fn all_locked(&self) -> impl Iterator<Item = LockedDependencyInfo> {
        self.as_json.locked.iter().map(|(name, dep)| {
            LockedDependencyInfo::new(name.as_str(), &dep.version, &dep.dependencies)
        })
    }

    pub(super) fn get_locked(&self, package: &str) -> Option<LockedDependencyInfo> {
        self.as_json
            .locked
            .get_key_value(package)
            .map(|(package, x)| LockedDependencyInfo::new(package, &x.version, &x.dependencies))
    }

    pub(super) fn add_dependency(&mut self, name: &str, version: DependencyRange) {
        self.as_json
            .dependencies
            .insert(name.to_owned(), VpmDependency { version });
        self.changed = true;
    }

    pub(super) fn add_locked(
        &mut self,
        name: &str,
        version: Version,
        dependencies: IndexMap<String, VersionRange>,
    ) {
        self.as_json.locked.insert(
            name.to_owned(),
            VpmLockedDependency {
                version,
                dependencies,
            },
        );
        self.changed = true;
    }

    pub(crate) fn remove_packages<'a>(&mut self, names: impl Iterator<Item = &'a str>) {
        for name in names {
            self.as_json.locked.remove(name);
            self.as_json.dependencies.remove(name);
        }
        self.changed = true;
    }

    pub(super) async fn save_to(&self, file: &Path) -> io::Result<()> {
        if self.changed {
            tokio::fs::write(file, &to_vec_pretty_os_eol(&self.as_json)?).await?;
        }
        Ok(())
    }
}

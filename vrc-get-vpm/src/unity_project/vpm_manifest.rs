use serde::{Deserialize, Serialize};
use serde_json::to_vec_pretty;
use std::collections::VecDeque;

use super::*;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AsJson {
    #[serde(default)]
    dependencies: IndexMap<String, VpmDependency>,
    #[serde(default)]
    locked: IndexMap<String, VpmLockedDependency>,
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

    pub(super) fn dependencies(&self) -> &IndexMap<String, VpmDependency> {
        &self.as_json.dependencies
    }

    pub(super) fn locked(&self) -> &IndexMap<String, VpmLockedDependency> {
        &self.as_json.locked
    }

    pub(super) fn add_dependency(&mut self, name: &str, dependency: VpmDependency) {
        self.as_json
            .dependencies
            .insert(name.to_owned(), dependency);
        self.changed = true;
    }

    pub(super) fn add_locked(&mut self, name: &str, dependency: VpmLockedDependency) {
        self.as_json.locked.insert(name.to_owned(), dependency);
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
            tokio::fs::write(file, &to_vec_pretty(&self.as_json)?).await?;
        }
        Ok(())
    }

    pub(crate) fn mark_and_sweep_packages(
        &mut self,
        unlocked: &[(String, Option<PackageJson>)],
    ) -> HashSet<String> {
        // mark
        let mut required_packages = HashSet::<&str>::new();
        for x in self.as_json.dependencies.keys() {
            required_packages.insert(x);
        }

        required_packages.extend(
            unlocked
                .iter()
                .filter_map(|(_, pkg)| pkg.as_ref())
                .flat_map(|x| x.vpm_dependencies().keys())
                .map(String::as_str),
        );

        let mut queue = required_packages.iter().copied().collect::<VecDeque<_>>();

        while let Some(dep_name) = queue.pop_back() {
            for dep_name in self
                .as_json
                .locked
                .get(dep_name)
                .into_iter()
                .flat_map(|dep| dep.dependencies.keys())
            {
                if required_packages.insert(dep_name) {
                    queue.push_front(dep_name);
                }
            }
        }

        // sweep
        let removing_packages = self
            .as_json
            .locked
            .keys()
            .filter(|&x| !required_packages.contains(x.as_str()))
            .cloned()
            .collect::<HashSet<_>>();

        //log::debug!("removing: {removing_packages:?}");

        for name in &removing_packages {
            self.as_json.locked.remove(name);
        }

        removing_packages
    }
}

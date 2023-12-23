use serde::Serialize;
use serde_json::{json, to_vec_pretty};

use super::*;

#[derive(Debug)]
pub(super) struct VpmManifest {
    json: JsonMap,
    dependencies: IndexMap<String, VpmDependency>,
    locked: IndexMap<String, VpmLockedDependency>,
    changed: bool,
}

impl VpmManifest {
    pub(super) fn new(json: JsonMap) -> serde_json::Result<Self> {
        Ok(Self {
            dependencies: from_value(
                json.get("dependencies")
                    .cloned()
                    .unwrap_or(Value::Object(JsonMap::new())),
            )?,
            locked: from_value(
                json.get("locked")
                    .cloned()
                    .unwrap_or(Value::Object(JsonMap::new())),
            )?,
            json,
            changed: false,
        })
    }

    pub(super) fn dependencies(&self) -> &IndexMap<String, VpmDependency> {
        &self.dependencies
    }

    pub(super) fn locked(&self) -> &IndexMap<String, VpmLockedDependency> {
        &self.locked
    }

    pub(super) fn add_dependency(&mut self, name: String, dependency: VpmDependency) {
        // update both parsed and non-parsed
        self.add_value("dependencies", &name, &dependency);
        self.dependencies.insert(name, dependency);
    }

    pub(super) fn add_locked(&mut self, name: &str, dependency: VpmLockedDependency) {
        // update both parsed and non-parsed
        self.add_value("locked", name, &dependency);
        self.locked.insert(name.to_string(), dependency);
    }

    pub(crate) fn remove_packages<'a>(&mut self, names: impl Iterator<Item = &'a str>) {
        for name in names {
            self.locked.remove(name);
            self.json
                .get_mut("locked")
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(name);
            self.dependencies.remove(name);
            self.json
                .get_mut("dependencies")
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(name);
        }
        self.changed = true;
    }

    fn add_value(&mut self, key0: &str, key1: &str, value: &impl Serialize) {
        let serialized = to_value(value).expect("serialize err");
        match self.json.get_mut(key0) {
            Some(Value::Object(obj)) => {
                obj.insert(key1.to_string(), serialized);
            }
            _ => {
                self.json.insert(key0.into(), json!({ key1: serialized }));
            }
        }
        self.changed = true;
    }

    pub(super) async fn save_to(&self, file: &Path) -> io::Result<()> {
        if !self.changed {
            return Ok(());
        }
        tokio::fs::write(file, &to_vec_pretty(&self.json)?).await?;
        Ok(())
    }

    pub(crate) fn mark_and_sweep_packages(
        &mut self,
        unlocked: &[(String, Option<PackageJson>)],
    ) -> HashSet<String> {
        // mark
        let mut required_packages = HashSet::<&str>::new();
        for x in self.dependencies.keys() {
            required_packages.insert(x);
        }

        required_packages.extend(
            unlocked
                .iter()
                .filter_map(|(_, pkg)| pkg.as_ref())
                .flat_map(|x| x.vpm_dependencies().keys())
                .map(String::as_str),
        );

        let mut added_prev = required_packages.iter().copied().collect_vec();

        while !added_prev.is_empty() {
            let mut added = Vec::<&str>::new();

            for dep_name in added_prev
                .into_iter()
                .filter_map(|name| self.locked.get(name))
                .flat_map(|dep| dep.dependencies.keys())
            {
                if required_packages.insert(dep_name) {
                    added.push(dep_name);
                }
            }

            added_prev = added;
        }

        // sweep
        let removing_packages = self
            .locked
            .keys()
            .filter(|&x| !required_packages.contains(x.as_str()))
            .cloned()
            .collect::<HashSet<_>>();

        //log::debug!("removing: {removing_packages:?}");

        for name in &removing_packages {
            self.locked.remove(name);
            self.json
                .get_mut("locked")
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(name);
        }

        removing_packages
    }
}

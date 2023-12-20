mod add_package;
mod package_resolution;
mod remove_package;

use crate::structs::manifest::{VpmDependency, VpmLockedDependency};
use crate::structs::package::PackageJson;
use crate::unity_project::vpm_manifest::VpmManifest;
use crate::utils::PathBufExt;
use crate::version::{UnityVersion, VersionRange};
use crate::{
    load_json_or_default, to_json_vec, Environment, JsonMap, PackageInfo, PackageSelector,
};
use futures::future::try_join_all;
use futures::prelude::*;
use indexmap::IndexMap;
use itertools::Itertools;
use serde_json::{from_value, to_value, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::{env, io};
use tokio::fs::{read_dir, remove_dir_all, DirEntry, File};
use tokio::io::AsyncReadExt;

// note: this module only declares basic small operations.
// there are module for each complex operations.

pub use add_package::{AddPackageErr, AddPackageRequest};
pub use remove_package::RemovePackageErr;

#[derive(Debug)]
pub struct UnityProject {
    /// path to project folder.
    project_dir: PathBuf,
    /// manifest.json
    manifest: VpmManifest,
    /// unity version parsed
    unity_version: Option<UnityVersion>,
    /// packages installed in the directory but not locked in vpm-manifest.json
    unlocked_packages: Vec<(String, Option<PackageJson>)>,
    installed_packages: HashMap<String, PackageJson>,
}

// basic lifecycle
impl UnityProject {
    pub async fn find_unity_project(unity_project: Option<PathBuf>) -> io::Result<UnityProject> {
        let unity_found = unity_project
            .ok_or(())
            .or_else(|_| UnityProject::find_unity_project_path())?;

        log::debug!(
            "initializing UnityProject with unity folder {}",
            unity_found.display()
        );

        let manifest = unity_found.join("Packages").joined("vpm-manifest.json");
        let vpm_manifest = VpmManifest::new(load_json_or_default(&manifest).await?)?;

        let mut installed_packages = HashMap::new();
        let mut unlocked_packages = vec![];

        let mut dir_reading = read_dir(unity_found.join("Packages")).await?;
        while let Some(dir_entry) = dir_reading.next_entry().await? {
            let read = Self::try_read_unlocked_package(dir_entry).await;
            let mut is_installed = false;
            if let Some(parsed) = &read.1 {
                if parsed.name == read.0 && vpm_manifest.locked().contains_key(&parsed.name) {
                    is_installed = true;
                }
            }
            if is_installed {
                installed_packages.insert(read.0, read.1.unwrap());
            } else {
                unlocked_packages.push(read);
            }
        }

        let unity_version = Self::try_read_unity_version(&unity_found).await;

        Ok(UnityProject {
            project_dir: unity_found,
            manifest: VpmManifest::new(load_json_or_default(&manifest).await?)?,
            unity_version,
            unlocked_packages,
            installed_packages,
        })
    }

    async fn try_read_unlocked_package(dir_entry: DirEntry) -> (String, Option<PackageJson>) {
        let package_path = dir_entry.path();
        let name = package_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let package_json_path = package_path.join("package.json");
        let parsed = load_json_or_default::<Option<PackageJson>>(&package_json_path)
            .await
            .ok()
            .flatten();
        (name, parsed)
    }

    fn find_unity_project_path() -> io::Result<PathBuf> {
        let mut candidate = env::current_dir()?;

        loop {
            candidate.push("Packages");
            candidate.push("vpm-manifest.json");

            if candidate.exists() {
                log::debug!("vpm-manifest.json found at {}", candidate.display());
                // if there's vpm-manifest.json, it's project path
                candidate.pop();
                candidate.pop();
                return Ok(candidate);
            }

            // replace vpm-manifest.json -> manifest.json
            candidate.pop();
            candidate.push("manifest.json");

            if candidate.exists() {
                log::debug!("manifest.json found at {}", candidate.display());
                // if there's manifest.json (which is manifest.json), it's project path
                candidate.pop();
                candidate.pop();
                return Ok(candidate);
            }

            // remove Packages/manifest.json
            candidate.pop();
            candidate.pop();

            log::debug!("Unity Project not found on {}", candidate.display());

            // go to parent dir
            if !candidate.pop() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Unity project Not Found",
                ));
            }
        }
    }

    async fn try_read_unity_version(unity_project: &Path) -> Option<UnityVersion> {
        let project_version_file = unity_project
            .join("ProjectSettings")
            .joined("ProjectVersion.txt");

        let mut project_version_file = match File::open(project_version_file).await {
            Ok(file) => file,
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                log::error!("ProjectVersion.txt not found");
                return None;
            }
            Err(e) => {
                log::error!("opening ProjectVersion.txt failed with error: {e}");
                return None;
            }
        };

        let mut buffer = String::new();

        if let Err(e) = project_version_file.read_to_string(&mut buffer).await {
            log::error!("reading ProjectVersion.txt failed with error: {e}");
            return None;
        };

        let Some((_, version_info)) = buffer.split_once("m_EditorVersion:") else {
            log::error!("m_EditorVersion not found in ProjectVersion.txt");
            return None
        };

        let version_info_end = version_info
            .find(|x: char| x == '\r' || x == '\n')
            .unwrap_or(version_info.len());
        let version_info = &version_info[..version_info_end];
        let version_info = version_info.trim();

        let Some(unity_version) = UnityVersion::parse(version_info) else {
            log::error!("failed to unity version in ProjectVersion.txt ({version_info})");
            return None
        };

        Some(unity_version)
    }

    pub async fn save(&mut self) -> io::Result<()> {
        self.manifest
            .save_to(
                &self
                    .project_dir
                    .join("Packages")
                    .joined("vpm-manifest.json"),
            )
            .await
    }
}

pub struct ResolveResult<'env> {
    installed_from_locked: Vec<PackageInfo<'env>>,
    installed_from_unlocked_dependencies: Vec<PackageInfo<'env>>,
}

impl<'env> ResolveResult<'env> {
    pub fn installed_from_locked(&self) -> &[PackageInfo<'env>] {
        &self.installed_from_locked
    }

    pub fn installed_from_unlocked_dependencies(&self) -> &[PackageInfo<'env>] {
        &self.installed_from_unlocked_dependencies
    }
}

impl UnityProject {
    pub async fn resolve<'env>(
        &mut self,
        env: &'env Environment,
    ) -> Result<ResolveResult<'env>, AddPackageErr> {
        // first, process locked dependencies
        let this = self as &Self;
        let packages_folder = &this.project_dir.join("Packages");
        let installed_from_locked = try_join_all(this.manifest.locked().into_iter().map(
            |(pkg, dep)| async move {
                let pkg = env
                    .find_package_by_name(pkg, PackageSelector::specific_version(&dep.version))
                    .unwrap_or_else(|| panic!("some package in manifest.json not found: {pkg}"));
                env.add_package(pkg, packages_folder).await?;
                Result::<_, AddPackageErr>::Ok(pkg)
            },
        ))
        .await?;

        let unlocked_names: HashSet<_> = self
            .unlocked_packages()
            .iter()
            .filter_map(|(_, pkg)| pkg.as_ref())
            .map(|x| x.name.as_str())
            .collect();

        // then, process dependencies of unlocked packages.
        let unlocked_dependencies = self
            .unlocked_packages
            .iter()
            .filter_map(|(_, pkg)| pkg.as_ref())
            .flat_map(|pkg| &pkg.vpm_dependencies)
            .filter(|(k, _)| !self.manifest.locked().contains_key(k.as_str()))
            .filter(|(k, _)| !unlocked_names.contains(k.as_str()))
            .map(|(k, v)| (k, v))
            .into_group_map()
            .into_iter()
            .map(|(pkg_name, ranges)| {
                env.find_package_by_name(
                    pkg_name,
                    PackageSelector::ranges_for(self.unity_version, &ranges),
                )
                .unwrap_or_else(|| {
                    panic!("some dependencies of unlocked package not found: {pkg_name}")
                })
            })
            .collect::<Vec<_>>();

        let allow_prerelease = unlocked_dependencies
            .iter()
            .any(|x| !x.version().pre.is_empty());

        let req = self
            .add_package_request(env, unlocked_dependencies, false, allow_prerelease)
            .await?;

        if !req.conflicts.is_empty() {
            let (conflict, mut deps) = req.conflicts.into_iter().next().unwrap();
            return Err(AddPackageErr::ConflictWithDependencies {
                conflict,
                dependency_name: deps.swap_remove(0),
            });
        }

        let installed_from_unlocked_dependencies = req.locked.clone();

        self.do_add_package_request(env, req).await?;

        Ok(ResolveResult {
            installed_from_locked,
            installed_from_unlocked_dependencies,
        })
    }

    /// Remove specified package from self project.
    ///
    /// This doesn't look packages not listed in vpm-maniefst.json.
    pub async fn mark_and_sweep(&mut self) -> io::Result<HashSet<String>> {
        let removed_packages = self
            .manifest
            .mark_and_sweep_packages(&self.unlocked_packages);

        try_join_all(removed_packages.iter().map(|name| {
            remove_dir_all(self.project_dir.join("Packages").joined(name)).map(|x| match x {
                Ok(()) => Ok(()),
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e),
            })
        }))
        .await?;

        Ok(removed_packages)
    }
}

// accessors
impl UnityProject {
    pub fn locked_packages(&self) -> &IndexMap<String, VpmLockedDependency> {
        return self.manifest.locked();
    }

    pub fn all_dependencies(
        &self,
    ) -> impl Iterator<Item = (&String, &IndexMap<String, VersionRange>)> {
        let dependencies_locked = self
            .manifest
            .locked()
            .into_iter()
            .map(|(name, dep)| (name, &dep.dependencies));

        let dependencies_unlocked = self
            .unlocked_packages
            .iter()
            .filter_map(|(_, json)| json.as_ref())
            .map(|x| (&x.name, &x.vpm_dependencies));

        dependencies_locked.chain(dependencies_unlocked)
    }

    pub fn unlocked_packages(&self) -> &[(String, Option<PackageJson>)] {
        &self.unlocked_packages
    }

    pub fn get_installed_package(&self, name: &str) -> Option<&PackageJson> {
        self.installed_packages.get(name)
    }

    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    pub fn unity_version(&self) -> Option<UnityVersion> {
        self.unity_version
    }
}

mod vpm_manifest {
    use serde::Serialize;
    use serde_json::json;
    use tokio::io::AsyncWriteExt;

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
            let mut file = File::create(file).await?;
            file.write_all(&to_json_vec(&self.json)?).await?;
            file.flush().await?;
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
                    .flat_map(|x| x.vpm_dependencies.keys())
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
                .cloned()
                .filter(|x| !required_packages.contains(x.as_str()))
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
}

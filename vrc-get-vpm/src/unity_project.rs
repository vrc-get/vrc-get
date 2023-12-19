use crate::structs::manifest::{VpmDependency, VpmLockedDependency};
use crate::structs::package::PackageJson;
use crate::unity_project::vpm_manifest::VpmManifest;
use crate::utils::{parse_hex_128, walk_dir, PathBufExt};
use crate::version::{UnityVersion, VersionRange};
use crate::{
    load_json_or_default, package_resolution, to_json_vec, unity_compatible, Environment, JsonMap,
    PackageInfo, PackageSelector,
};
use futures::future::{join3, join_all, try_join_all};
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use indexmap::IndexMap;
use itertools::Itertools;
use serde_json::{from_value, to_value, Value};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::{env, fmt, io};
use tokio::fs::{metadata, read_dir, remove_dir_all, DirEntry, File};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

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

    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    pub fn unity_version(&self) -> Option<UnityVersion> {
        self.unity_version
    }
}

impl UnityProject {
    pub async fn add_package_request<'env>(
        &self,
        env: &'env Environment,
        mut packages: Vec<PackageInfo<'env>>,
        to_dependencies: bool,
        allow_prerelease: bool,
    ) -> Result<AddPackageRequest<'env>, AddPackageErr> {
        packages.retain(|pkg| {
            self.manifest
                .dependencies()
                .get(pkg.name())
                .map(|dep| dep.version.matches(pkg.version()))
                .unwrap_or(true)
        });

        // if same or newer requested package is in locked dependencies,
        // just add requested version into dependencies
        let mut dependencies = vec![];
        let mut adding_packages = Vec::with_capacity(packages.len());

        for request in packages {
            let update = self
                .manifest
                .locked()
                .get(request.name())
                .map(|dep| dep.version < *request.version())
                .unwrap_or(true);

            if to_dependencies {
                dependencies.push((
                    request.name(),
                    VpmDependency::new(request.version().clone()),
                ));
            }

            if update {
                adding_packages.push(request);
            }
        }

        if adding_packages.len() == 0 {
            // early return:
            return Ok(AddPackageRequest {
                dependencies,
                locked: vec![],
                legacy_files: vec![],
                legacy_folders: vec![],
                legacy_packages: vec![],
                conflicts: HashMap::new(),
                unity_conflicts: vec![],
            });
        }

        let result = package_resolution::collect_adding_packages(
            self.manifest.dependencies(),
            self.manifest.locked(),
            self.unity_version(),
            env,
            adding_packages,
            allow_prerelease,
        )?;

        let legacy_packages = result
            .found_legacy_packages
            .into_iter()
            .filter(|name| self.manifest.locked().contains_key(name))
            .collect();

        let (legacy_files, legacy_folders) = self.collect_legacy_assets(&result.new_packages).await;

        let unity_conflicts = if let Some(unity) = self.unity_version {
            result
                .new_packages
                .iter()
                .filter(|pkg| !unity_compatible(pkg, unity))
                .map(|pkg| pkg.name().to_owned())
                .collect()
        } else {
            vec![]
        };

        return Ok(AddPackageRequest {
            dependencies,
            locked: result.new_packages,
            conflicts: result.conflicts,
            unity_conflicts,
            legacy_files,
            legacy_folders,
            legacy_packages,
        });
    }

    async fn collect_legacy_assets(
        &self,
        packages: &[PackageInfo<'_>],
    ) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let folders = packages
            .iter()
            .flat_map(|x| &x.package_json().legacy_folders)
            .map(|(path, guid)| (path, guid, false));
        let files = packages
            .iter()
            .flat_map(|x| &x.package_json().legacy_files)
            .map(|(path, guid)| (path, guid, true));
        let assets = folders.chain(files).collect::<Vec<_>>();

        enum LegacyInfo {
            FoundFile(PathBuf),
            FoundFolder(PathBuf),
            NotFound,
            GuidFile(GUID),
            GuidFolder(GUID),
        }
        use LegacyInfo::*;

        #[derive(Copy, Clone, Hash, Eq, PartialEq)]
        struct GUID([u8; 16]);

        fn try_parse_guid(guid: &str) -> Option<GUID> {
            Some(GUID(parse_hex_128(guid.as_bytes().try_into().ok()?)?))
        }

        let mut futures = pin!(assets
            .into_iter()
            .map(|(path, guid, is_file)| async move {
                // some packages uses '/' as path separator.
                let path = PathBuf::from(path.replace('\\', "/"));
                // for security, deny absolute path.
                if path.has_root() {
                    return NotFound;
                }
                let path = self.project_dir.join(path);
                if metadata(&path)
                    .await
                    .map(|x| x.is_file() == is_file)
                    .unwrap_or(false)
                {
                    if is_file {
                        FoundFile(path)
                    } else {
                        FoundFolder(path)
                    }
                } else {
                    if let Some(guid) = guid.as_deref().and_then(try_parse_guid) {
                        if is_file {
                            GuidFile(guid)
                        } else {
                            GuidFolder(guid)
                        }
                    } else {
                        NotFound
                    }
                }
            })
            .collect::<FuturesUnordered<_>>());

        let mut found_files = HashSet::new();
        let mut found_folders = HashSet::new();
        let mut find_guids = HashMap::new();

        while let Some(info) = futures.next().await {
            match info {
                FoundFile(path) => {
                    found_files.insert(path.strip_prefix(&self.project_dir).unwrap().to_owned());
                }
                FoundFolder(path) => {
                    found_folders.insert(path.strip_prefix(&self.project_dir).unwrap().to_owned());
                }
                NotFound => (),
                GuidFile(guid) => {
                    find_guids.insert(guid, true);
                }
                GuidFolder(guid) => {
                    find_guids.insert(guid, false);
                }
            }
        }

        if find_guids.len() != 0 {
            async fn get_guid(entry: DirEntry) -> Option<(GUID, bool, PathBuf)> {
                let path = entry.path();
                if path.extension() != Some(OsStr::new("meta"))
                    || !entry.file_type().await.ok()?.is_file()
                {
                    return None;
                }
                let mut file = BufReader::new(File::open(&path).await.ok()?);
                let mut buffer = String::new();
                while file.read_line(&mut buffer).await.ok()? != 0 {
                    let line = buffer.as_str();
                    if let Some(guid) = line.strip_prefix("guid: ") {
                        // current line should be line for guid.
                        if let Some(guid) = try_parse_guid(guid.trim()) {
                            // remove .meta extension
                            let mut path = path;
                            path.set_extension("");
                            let is_file = metadata(&path).await.ok()?.is_file();
                            return Some((guid, is_file, path));
                        }
                    }

                    buffer.clear()
                }

                None
            }

            let mut stream = pin!(walk_dir([
                self.project_dir.join("Packages"),
                self.project_dir.join("Assets")
            ])
            .filter_map(get_guid));

            while let Some((guid, is_file_actual, path)) = stream.next().await {
                if let Some(&is_file) = find_guids.get(&guid) {
                    if is_file_actual == is_file {
                        find_guids.remove(&guid);
                        if is_file {
                            found_files
                                .insert(path.strip_prefix(&self.project_dir).unwrap().to_owned());
                        } else {
                            found_folders
                                .insert(path.strip_prefix(&self.project_dir).unwrap().to_owned());
                        }
                    }
                }
            }
        }

        (
            found_files.into_iter().collect(),
            found_folders.into_iter().collect(),
        )
    }

    pub async fn do_add_package_request<'env>(
        &mut self,
        env: &'env Environment,
        request: AddPackageRequest<'env>,
    ) -> io::Result<()> {
        // first, add to dependencies
        for x in request.dependencies {
            self.manifest.add_dependency(x.0.to_owned(), x.1);
        }

        // then, do install packages
        self.do_add_packages_to_locked(env, &request.locked).await?;

        let project_dir = &self.project_dir;

        // finally, try to remove legacy assets
        self.manifest
            .remove_packages(request.legacy_packages.iter().map(|x| x.as_str()));
        join3(
            join_all(
                request
                    .legacy_files
                    .into_iter()
                    .map(|x| remove_file(x, project_dir)),
            ),
            join_all(
                request
                    .legacy_folders
                    .into_iter()
                    .map(|x| remove_folder(x, project_dir)),
            ),
            join_all(
                request
                    .legacy_packages
                    .into_iter()
                    .map(|x| remove_package(x, project_dir)),
            ),
        )
        .await;

        async fn remove_meta_file(path: PathBuf) {
            let mut building = path.into_os_string();
            building.push(".meta");
            let meta = PathBuf::from(building);

            if let Some(err) = tokio::fs::remove_file(&meta).await.err() {
                if !matches!(err.kind(), io::ErrorKind::NotFound) {
                    log::error!("error removing legacy asset at {}: {}", meta.display(), err);
                }
            }
        }

        async fn remove_file(path: PathBuf, project_dir: &Path) {
            let path = project_dir.join(path);
            if let Some(err) = tokio::fs::remove_file(&path).await.err() {
                log::error!("error removing legacy asset at {}: {}", path.display(), err);
            }
            remove_meta_file(path).await;
        }

        async fn remove_folder(path: PathBuf, project_dir: &Path) {
            let path = project_dir.join(path);
            if let Some(err) = tokio::fs::remove_dir_all(&path).await.err() {
                log::error!("error removing legacy asset at {}: {}", path.display(), err);
            }
            remove_meta_file(path).await;
        }

        async fn remove_package(name: String, project_dir: &Path) {
            let folder = project_dir.join("Packages").joined(name);
            if let Some(err) = tokio::fs::remove_dir_all(&folder).await.err() {
                log::error!(
                    "error removing legacy package at {}: {}",
                    folder.display(),
                    err
                );
            }
        }

        Ok(())
    }

    async fn do_add_packages_to_locked(
        &mut self,
        env: &Environment,
        packages: &[PackageInfo<'_>],
    ) -> io::Result<()> {
        // then, lock all dependencies
        for pkg in packages.iter() {
            self.manifest.add_locked(
                &pkg.name(),
                VpmLockedDependency::new(pkg.version().clone(), pkg.vpm_dependencies().clone()),
            );
        }

        let packages_folder = self.project_dir.join("Packages");

        // resolve all packages
        let futures = packages
            .iter()
            .map(|x| env.add_package(*x, &packages_folder))
            .collect::<Vec<_>>();
        try_join_all(futures).await?;

        Ok(())
    }

    /// Remove specified package from self project.
    ///
    /// This doesn't look packages not listed in vpm-maniefst.json.
    pub async fn remove(&mut self, names: &[&str]) -> Result<(), RemovePackageErr> {
        use RemovePackageErr::*;

        // check for existence

        let mut repos = Vec::with_capacity(names.len());
        let mut not_founds = Vec::new();
        for name in names.into_iter().copied() {
            if let Some(x) = self.manifest.locked().get(name) {
                repos.push(x);
            } else {
                not_founds.push(name.to_owned());
            }
        }

        if !not_founds.is_empty() {
            return Err(NotInstalled(not_founds));
        }

        // check for conflicts: if some package requires some packages to be removed, it's conflict.

        let conflicts = self
            .all_dependencies()
            .filter(|(name, _)| !names.contains(&name.as_str()))
            .filter(|(_, dep)| names.into_iter().any(|x| dep.contains_key(*x)))
            .map(|(name, _)| String::from(name))
            .collect::<Vec<_>>();

        if !conflicts.is_empty() {
            return Err(ConflictsWith(conflicts));
        }

        // there's no conflicts. So do remove

        self.manifest.remove_packages(names.iter().copied());
        try_join_all(names.into_iter().map(|name| {
            remove_dir_all(self.project_dir.join("Packages").joined(name)).map(|x| match x {
                Ok(()) => Ok(()),
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e),
            })
        }))
        .await?;

        Ok(())
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
                    .find_package_by_name(&pkg, PackageSelector::specific_version(&dep.version))
                    .unwrap_or_else(|| panic!("some package in manifest.json not found: {pkg}"));
                env.add_package(pkg, packages_folder).await?;
                Result::<_, AddPackageErr>::Ok(pkg)
            },
        ))
        .await?;

        let unlocked_names: HashSet<_> = self
            .unlocked_packages()
            .into_iter()
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
            .add_package_request(&env, unlocked_dependencies, false, allow_prerelease)
            .await?;

        if req.conflicts.len() != 0 {
            let (conflict, mut deps) = req.conflicts.into_iter().next().unwrap();
            return Err(AddPackageErr::ConflictWithDependencies {
                conflict,
                dependency_name: deps.swap_remove(0),
            });
        }

        let installed_from_unlocked_dependencies = req.locked.clone();

        self.do_add_package_request(&env, req).await?;

        Ok(ResolveResult {
            installed_from_locked,
            installed_from_unlocked_dependencies,
        })
    }
}

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

        return dependencies_locked.chain(dependencies_unlocked);
    }

    pub fn unlocked_packages(&self) -> &[(String, Option<PackageJson>)] {
        &self.unlocked_packages
    }

    pub fn get_installed_package(&self, name: &str) -> Option<&PackageJson> {
        self.installed_packages.get(name)
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
                .map(|x| x.clone())
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

pub struct AddPackageRequest<'env> {
    dependencies: Vec<(&'env str, VpmDependency)>,
    locked: Vec<PackageInfo<'env>>,
    legacy_files: Vec<PathBuf>,
    legacy_folders: Vec<PathBuf>,
    legacy_packages: Vec<String>,
    conflicts: HashMap<String, Vec<String>>,
    unity_conflicts: Vec<String>,
}

impl<'env> AddPackageRequest<'env> {
    pub fn locked(&self) -> &[PackageInfo<'env>] {
        &self.locked
    }

    pub fn dependencies(&self) -> &[(&'env str, VpmDependency)] {
        &self.dependencies
    }

    pub fn legacy_files(&self) -> &[PathBuf] {
        &self.legacy_files
    }

    pub fn legacy_folders(&self) -> &[PathBuf] {
        &self.legacy_folders
    }

    pub fn legacy_packages(&self) -> &[String] {
        &self.legacy_packages
    }

    pub fn conflicts(&self) -> &HashMap<String, Vec<String>> {
        &self.conflicts
    }

    pub fn unity_conflicts(&self) -> &Vec<String> {
        &self.unity_conflicts
    }
}

#[derive(Debug)]
pub enum AddPackageErr {
    Io(io::Error),
    ConflictWithDependencies {
        /// conflicting package name
        conflict: String,
        /// the name of locked package
        dependency_name: String,
    },
    DependencyNotFound {
        dependency_name: String,
    },
}

impl fmt::Display for AddPackageErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddPackageErr::Io(ioerr) => fmt::Display::fmt(ioerr, f),
            AddPackageErr::ConflictWithDependencies {
                conflict,
                dependency_name,
            } => write!(f, "{conflict} conflicts with {dependency_name}"),
            AddPackageErr::DependencyNotFound { dependency_name } => write!(
                f,
                "Package {dependency_name} (maybe dependencies of the package) not found"
            ),
        }
    }
}

impl std::error::Error for AddPackageErr {}

impl From<io::Error> for AddPackageErr {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug)]
pub enum RemovePackageErr {
    Io(io::Error),
    NotInstalled(Vec<String>),
    ConflictsWith(Vec<String>),
}

impl fmt::Display for RemovePackageErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use RemovePackageErr::*;
        match self {
            Io(ioerr) => fmt::Display::fmt(ioerr, f),
            NotInstalled(names) => {
                f.write_str("the following packages are not installed: ")?;
                let mut iter = names.iter();
                f.write_str(iter.next().unwrap())?;
                while let Some(name) = iter.next() {
                    f.write_str(", ")?;
                    f.write_str(name)?;
                }
                Ok(())
            }
            ConflictsWith(names) => {
                f.write_str("removing packages conflicts with the following packages: ")?;
                let mut iter = names.iter();
                f.write_str(iter.next().unwrap())?;
                while let Some(name) = iter.next() {
                    f.write_str(", ")?;
                    f.write_str(name)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for RemovePackageErr {}

impl From<io::Error> for RemovePackageErr {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

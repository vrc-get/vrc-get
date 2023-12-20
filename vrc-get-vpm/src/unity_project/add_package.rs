use crate::add_package::add_package;
use crate::structs::manifest::{VpmDependency, VpmLockedDependency};
use crate::unity_project::package_resolution;
use crate::utils::{parse_hex_128, walk_dir, PathBufExt};
use crate::{unity_compatible, Environment, PackageInfo, UnityProject};
use futures::future::{join3, join_all, try_join_all};
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::{fmt, io};
use tokio::fs::{metadata, DirEntry, File};
use tokio::io::{AsyncBufReadExt, BufReader};

/// Represents Packages to be added and folders / packages to be removed
///
/// In vrc-get, Adding package is divided into two phases:
/// - Collect modifications
/// - Apply collected changes
///
/// This is done to ask users before removing packages
pub struct AddPackageRequest<'env> {
    dependencies: Vec<(&'env str, VpmDependency)>,
    pub(crate) locked: Vec<PackageInfo<'env>>,
    legacy_files: Vec<PathBuf>,
    legacy_folders: Vec<PathBuf>,
    legacy_packages: Vec<String>,
    pub(crate) conflicts: HashMap<String, Vec<String>>, // used by resolve
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

// adding package
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

        if adding_packages.is_empty() {
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

        Ok(AddPackageRequest {
            dependencies,
            locked: result.new_packages,
            conflicts: result.conflicts,
            unity_conflicts,
            legacy_files,
            legacy_folders,
            legacy_packages,
        })
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
            GuidFile(Guid),
            GuidFolder(Guid),
        }
        use LegacyInfo::*;

        #[derive(Copy, Clone, Hash, Eq, PartialEq)]
        struct Guid([u8; 16]);

        fn try_parse_guid(guid: &str) -> Option<Guid> {
            Some(Guid(parse_hex_128(guid.as_bytes().try_into().ok()?)?))
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
                } else if let Some(guid) = guid.as_deref().and_then(try_parse_guid) {
                    if is_file {
                        GuidFile(guid)
                    } else {
                        GuidFolder(guid)
                    }
                } else {
                    NotFound
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

        if !find_guids.is_empty() {
            async fn get_guid(entry: DirEntry) -> Option<(Guid, bool, PathBuf)> {
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
                pkg.name(),
                VpmLockedDependency::new(pkg.version().clone(), pkg.vpm_dependencies().clone()),
            );
        }

        let packages_folder = self.project_dir.join("Packages");

        // resolve all packages
        let futures = packages
            .iter()
            .map(|package| {
                add_package(
                    &env.global_dir,
                    env.http.as_ref(),
                    *package,
                    &packages_folder,
                )
            })
            .collect::<Vec<_>>();
        try_join_all(futures).await?;

        Ok(())
    }
}

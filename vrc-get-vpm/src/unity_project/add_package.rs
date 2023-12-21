use crate::structs::manifest::{VpmDependency, VpmLockedDependency};
use crate::traits::RemotePackageDownloader;
use crate::unity_project::package_resolution;
use crate::utils::{copy_recursive, extract_zip, walk_dir_relative, PathBufExt, WalkDirEntry};
use crate::{unity_compatible, PackageCollection, PackageInfo, PackageInfoInner, UnityProject};
use futures::future::{join3, join_all, try_join_all};
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use hex::FromHex;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::{fmt, io};
use tokio::fs::{metadata, remove_dir_all, File};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::compat::*;

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
#[non_exhaustive]
pub enum AddPackageErr {
    DependencyNotFound { dependency_name: String },
}

impl fmt::Display for AddPackageErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddPackageErr::DependencyNotFound { dependency_name } => write!(
                f,
                "Package {dependency_name} (maybe dependencies of the package) not found"
            ),
        }
    }
}

impl std::error::Error for AddPackageErr {}

// adding package
impl UnityProject {
    /// Creates a new `AddPackageRequest` to add the specified packages.
    ///
    /// You should call `do_add_package_request` to apply the changes after confirming to the user.
    pub async fn add_package_request<'env>(
        &self,
        env: &'env impl PackageCollection,
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

        let (legacy_files, legacy_folders) =
            Self::collect_legacy_assets(&self.project_dir, &result.new_packages).await;

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
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
struct Guid([u8; 16]);

impl Guid {
    fn parse(guid: &str) -> Option<Guid> {
        FromHex::from_hex(guid).ok().map(Guid)
    }
}

struct DefinedLegacyInfo<'a> {
    path: &'a str,
    guid: Option<Guid>,
    is_file: bool,
}

impl<'a> DefinedLegacyInfo<'a> {
    fn new_file(path: &'a str, guid: Option<Guid>) -> Self {
        Self {
            path,
            guid,
            is_file: true,
        }
    }

    fn new_dir(path: &'a str, guid: Option<Guid>) -> Self {
        Self {
            path,
            guid,
            is_file: false,
        }
    }
}

enum LegacySearchResult {
    FoundWithPath(PathBuf, bool),
    SearchWithGuid(Guid, bool),
}

impl UnityProject {
    async fn collect_legacy_assets(
        project_dir: &Path,
        packages: &[PackageInfo<'_>],
    ) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let folders = packages
            .iter()
            .flat_map(|x| &x.package_json().legacy_folders)
            .map(|(path, guid)| {
                DefinedLegacyInfo::new_dir(path, guid.as_deref().and_then(Guid::parse))
            });
        let files = packages
            .iter()
            .flat_map(|x| &x.package_json().legacy_files)
            .map(|(path, guid)| {
                DefinedLegacyInfo::new_file(path, guid.as_deref().and_then(Guid::parse))
            });
        let assets = folders.chain(files);

        let (mut found_files, mut found_folders, find_guids) =
            Self::find_legacy_assets_by_path(project_dir, assets).await;

        if !find_guids.is_empty() {
            Self::find_legacy_assets_by_guid(
                project_dir,
                find_guids,
                &mut found_files,
                &mut found_folders,
            )
            .await;
        }

        (
            found_files.into_iter().collect(),
            found_folders.into_iter().collect(),
        )
    }

    async fn find_legacy_assets_by_path(
        project_dir: &Path,
        assets: impl Iterator<Item = DefinedLegacyInfo<'_>>,
    ) -> (HashSet<PathBuf>, HashSet<PathBuf>, HashMap<Guid, bool>) {
        use LegacySearchResult::*;

        let mut futures = pin!(assets
            .map(|info| async move {
                // some packages uses '/' as path separator.
                let relative_path = PathBuf::from(info.path.replace('\\', "/"));
                // for security, deny absolute path.
                if relative_path.is_absolute() {
                    return None;
                }
                if metadata(project_dir.join(&relative_path))
                    .await
                    .map(|x| x.is_file() == info.is_file)
                    .unwrap_or(false)
                {
                    Some(FoundWithPath(relative_path, info.is_file))
                } else if let Some(guid) = info.guid {
                    Some(SearchWithGuid(guid, info.is_file))
                } else {
                    None
                }
            })
            .collect::<FuturesUnordered<_>>());

        let mut found_files = HashSet::new();
        let mut found_folders = HashSet::new();
        let mut find_guids = HashMap::new();

        while let Some(info) = futures.next().await {
            match info {
                Some(FoundWithPath(relative_path, true)) => {
                    found_files.insert(relative_path);
                }
                Some(FoundWithPath(relative_path, false)) => {
                    found_folders.insert(relative_path);
                }
                Some(SearchWithGuid(guid, is_file)) => {
                    find_guids.insert(guid, is_file);
                }
                None => (),
            }
        }

        (found_files, found_folders, find_guids)
    }

    async fn try_parse_meta(path: &Path) -> Option<Guid> {
        let mut file = BufReader::new(File::open(&path).await.ok()?);
        let mut buffer = String::new();
        while file.read_line(&mut buffer).await.ok()? != 0 {
            let line = buffer.as_str();
            if let Some(guid) = line.strip_prefix("guid: ") {
                // current line should be line for guid.
                return Guid::parse(guid.trim());
            }

            buffer.clear()
        }
        None
    }

    async fn find_legacy_assets_by_guid(
        project_dir: &Path,
        mut find_guids: HashMap<Guid, bool>,
        found_files: &mut HashSet<PathBuf>,
        found_folders: &mut HashSet<PathBuf>,
    ) {
        async fn get_guid(entry: WalkDirEntry) -> Option<(Guid, bool, PathBuf)> {
            let path = entry.path();
            if path.extension() != Some(OsStr::new("meta")) {
                None
            } else if let Some(guid) = UnityProject::try_parse_meta(&path).await {
                // remove .meta extension
                let mut path = path;
                path.set_extension("");

                let is_file = metadata(&path).await.ok()?.is_file();
                Some((guid, is_file, entry.relative))
            } else {
                None
            }
        }

        let mut stream = pin!(walk_dir_relative(
            project_dir,
            [PathBuf::from("Packages"), PathBuf::from("Assets")]
        )
        .filter_map(get_guid));

        while let Some((guid, is_file_actual, relative)) = stream.next().await {
            if let Some(&is_file) = find_guids.get(&guid) {
                if is_file_actual == is_file {
                    find_guids.remove(&guid);
                    if is_file {
                        found_files.insert(relative);
                    } else {
                        found_folders.insert(relative);
                    }
                }
            }
        }
    }
}

impl UnityProject {
    /// Applies the changes specified in `AddPackageRequest` to the project.
    pub async fn do_add_package_request<'env>(
        &mut self,
        env: &'env impl RemotePackageDownloader,
        request: AddPackageRequest<'env>,
    ) -> io::Result<()> {
        // first, add to dependencies
        for x in request.dependencies {
            self.manifest.add_dependency(x.0.to_owned(), x.1);
        }

        // then, lock all dependencies
        for pkg in request.locked.iter() {
            self.manifest.add_locked(
                pkg.name(),
                VpmLockedDependency::new(pkg.version().clone(), pkg.vpm_dependencies().clone()),
            );
        }

        // then, do install packages
        self.install_packages(env, &request.locked).await?;

        // finally, try to remove legacy assets
        self.manifest
            .remove_packages(request.legacy_packages.iter().map(|x| x.as_str()));

        Self::remove_legacy_assets(
            &self.project_dir,
            request.legacy_files.iter().map(PathBuf::as_path),
            request.legacy_folders.iter().map(PathBuf::as_path),
            request.legacy_packages.iter().map(String::as_str),
        )
        .await;

        Ok(())
    }

    async fn remove_legacy_assets(
        project_dir: &Path,
        legacy_files: impl Iterator<Item = &Path>,
        legacy_folders: impl Iterator<Item = &Path>,
        legacy_packages: impl Iterator<Item = &str>,
    ) {
        join3(
            join_all(legacy_files.map(|relative| async move {
                remove_file(project_dir.join(relative), true).await;
            })),
            join_all(legacy_folders.map(|relative| async move {
                remove_folder(project_dir.join(relative), true).await;
            })),
            join_all(legacy_packages.map(|name| async move {
                remove_folder(project_dir.join("Packages").joined(name), false).await;
            })),
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

        async fn remove_file(path: PathBuf, with_meta: bool) {
            if let Some(err) = tokio::fs::remove_file(&path).await.err() {
                log::error!("error removing legacy asset at {}: {}", path.display(), err);
            }
            if with_meta {
                remove_meta_file(path).await;
            }
        }

        async fn remove_folder(path: PathBuf, with_meta: bool) {
            if let Some(err) = tokio::fs::remove_dir_all(&path).await.err() {
                log::error!("error removing legacy asset at {}: {}", path.display(), err);
            }
            if with_meta {
                remove_meta_file(path).await;
            }
        }
    }

    async fn install_packages(
        &mut self,
        env: &impl RemotePackageDownloader,
        packages: &[PackageInfo<'_>],
    ) -> io::Result<()> {
        let packages_folder = self.project_dir.join("Packages");

        // resolve all packages
        try_join_all(
            packages
                .iter()
                .map(|package| add_package(env, *package, &packages_folder)),
        )
        .await?;

        Ok(())
    }
}

pub(crate) async fn add_package(
    remote_source: &impl RemotePackageDownloader,
    package: PackageInfo<'_>,
    target_packages_folder: &Path,
) -> io::Result<()> {
    log::debug!("adding package {}", package.name());
    let dest_folder = target_packages_folder.join(package.name());
    match package.inner {
        PackageInfoInner::Remote(package, user_repo) => {
            let zip_file = remote_source.get_package(user_repo, package).await?;

            // remove dest folder before extract if exists
            remove_dir_all(&dest_folder).await.ok();
            extract_zip(zip_file.compat(), &dest_folder).await?;

            Ok(())
        }
        PackageInfoInner::Local(_, path) => {
            remove_dir_all(&dest_folder).await.ok();
            copy_recursive(path.to_owned(), dest_folder).await?;
            Ok(())
        }
    }
}

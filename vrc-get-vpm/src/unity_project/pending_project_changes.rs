use crate::unity_project::add_package::add_package;
use crate::unity_project::find_legacy_assets::collect_legacy_assets;
use crate::utils::PathBufExt;
use crate::version::DependencyRange;
use crate::{unity_compatible, PackageInfo, RemotePackageDownloader, UnityProject};
use either::Either;
use futures::future::{join3, join_all, try_join_all};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

/// Represents Packages to be added and folders / packages to be removed
///
/// In vrc-get, Adding package is divided into two phases:
/// - Collect modifications
/// - Apply collected changes
///
/// This is done to ask users before removing packages
pub struct PendingProjectChanges<'env> {
    pub(crate) package_changes: HashMap<String, PackageChange<'env>>,

    pub(crate) remove_legacy_files: Vec<PathBuf>,
    pub(crate) remove_legacy_folders: Vec<PathBuf>,

    pub(crate) conflicts: HashMap<String, ConflictInfo>,
}

#[non_exhaustive]
pub enum PackageChange<'env> {
    Install(Install<'env>),
    Remove(Remove<'env>),
}

impl<'env> PackageChange<'env> {
    pub fn as_install(&self) -> Option<&Install<'env>> {
        match self {
            PackageChange::Install(x) => Some(x),
            PackageChange::Remove(_) => None,
        }
    }

    pub fn as_rmeove(&self) -> Option<&Remove<'env>> {
        match self {
            PackageChange::Install(_) => None,
            PackageChange::Remove(x) => Some(x),
        }
    }
}

pub struct Install<'env> {
    package: Option<PackageInfo<'env>>,
    add_to_locked: bool,
    to_dependencies: Option<DependencyRange>,
}

impl<'env> Install<'env> {
    pub fn install_package(&self) -> Option<PackageInfo<'env>> {
        self.package
    }
}

pub struct Remove<'env> {
    reason: RemoveReason,
    _phantom: PhantomData<&'env ()>,
}

impl<'env> Remove<'env> {
    pub fn reason(&self) -> RemoveReason {
        self.reason
    }
}

#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RemoveReason {
    Requested,
    Legacy,
    Unused,
}

#[derive(Default)]
pub struct ConflictInfo {
    conflicts_packages: Vec<String>,
    conflicts_with_unity: bool,
}

impl ConflictInfo {
    pub fn conflicting_packages(&self) -> &[String] {
        self.conflicts_packages.as_slice()
    }

    pub fn conflicts_with_unity(&self) -> bool {
        self.conflicts_with_unity
    }
}

pub(crate) struct Builder<'env> {
    package_changes: HashMap<String, PackageChange<'env>>,
    conflicts: HashMap<String, ConflictInfo>,
}

impl<'env> Builder<'env> {
    pub fn new() -> Self {
        Self {
            package_changes: HashMap::new(),
            conflicts: HashMap::new(),
        }
    }

    pub fn add_to_dependencies(&mut self, name: String, version: DependencyRange) -> &mut Self {
        match self.package_changes.entry(name) {
            Entry::Occupied(mut e) => match e.get_mut() {
                PackageChange::Install(e) => {
                    if e.to_dependencies.is_none() {
                        e.to_dependencies = Some(version);
                    } else {
                        panic!("INTERNAL ERROR: already add_to_dependencies");
                    }
                }
                PackageChange::Remove(_) => {
                    panic!("INTERNAL ERROR: add_to_dependencies for removed");
                }
            },
            Entry::Vacant(e) => {
                e.insert(PackageChange::Install(Install {
                    package: None,
                    add_to_locked: false,
                    to_dependencies: Some(version),
                }));
            }
        }

        self
    }

    pub fn install_locked(&mut self, info: PackageInfo<'env>) -> &mut Self {
        match self.package_changes.entry(info.name().to_owned()) {
            Entry::Occupied(mut e) => match e.get_mut() {
                PackageChange::Install(e) => {
                    if e.package.is_none() {
                        e.package = Some(info);
                        e.add_to_locked = true;
                    } else {
                        panic!("INTERNAL ERROR: already install");
                    }
                }
                PackageChange::Remove(_) => {
                    panic!("INTERNAL ERROR: install for removed");
                }
            },
            Entry::Vacant(e) => {
                e.insert(PackageChange::Install(Install {
                    package: Some(info),
                    add_to_locked: true,
                    to_dependencies: None,
                }));
            }
        }
        self
    }

    pub fn conflicts(&mut self, name: String, conflict: &[String]) -> &mut Self {
        self.conflicts
            .entry(name)
            .or_default()
            .conflicts_packages
            .extend_from_slice(conflict);
        self
    }

    pub fn conflicts_unity(&mut self, name: String) -> &mut Self {
        self.conflicts.entry(name).or_default().conflicts_with_unity = true;
        self
    }

    pub fn remove(&mut self, name: String, reason: RemoveReason) -> &mut Self {
        match self.package_changes.entry(name) {
            Entry::Occupied(mut e) => match e.get_mut() {
                PackageChange::Install(_) => {
                    panic!("INTERNAL ERROR: remove for installed");
                }
                PackageChange::Remove(e) => {
                    if e.reason != reason {
                        panic!("INTERNAL ERROR: already remove");
                    }
                }
            },
            Entry::Vacant(e) => {
                e.insert(PackageChange::Remove(Remove {
                    reason,
                    _phantom: PhantomData,
                }));
            }
        }
        self
    }

    pub fn build_no_resolve(self) -> PendingProjectChanges<'env> {
        for change in self.package_changes.values() {
            match change {
                PackageChange::Install(change) => {
                    if change.package.is_some() {
                        panic!("INTERNAL ERROR: install package requires resolve")
                    }
                }
                PackageChange::Remove(_) => {
                    panic!("INTERNAL ERROR: remove requires resolve")
                }
            }
        }

        PendingProjectChanges {
            package_changes: self.package_changes,
            conflicts: self.conflicts,

            remove_legacy_files: vec![],
            remove_legacy_folders: vec![],
        }
    }

    pub async fn build_resolve(
        mut self,
        unity_project: &UnityProject,
    ) -> PendingProjectChanges<'env> {
        let installs = Vec::from_iter(
            self.package_changes
                .values()
                .filter_map(|x| x.as_install())
                .filter(|x| x.add_to_locked)
                .map(|x| x.package.unwrap()),
        );

        if let Some(unity) = unity_project.unity_version {
            for package in installs
                .iter()
                .filter(|pkg| !unity_compatible(pkg, unity))
                .map(|pkg| pkg.name().to_owned())
            {
                self.conflicts_unity(package);
            }
        }

        self.mark_and_sweep_packages(unity_project);

        let legacy_assets = collect_legacy_assets(unity_project.project_dir(), &installs).await;

        PendingProjectChanges {
            package_changes: self.package_changes,
            conflicts: self.conflicts,

            remove_legacy_files: legacy_assets.files,
            remove_legacy_folders: legacy_assets.folders,
        }
    }

    fn mark_and_sweep_packages(&mut self, unity_project: &UnityProject) {
        fn mark_recursive<'a, F, I>(
            entrypoint: impl Iterator<Item = &'a str>,
            get_dependencies: F,
        ) -> HashSet<&'a str>
        where
            F: Fn(&'a str) -> I,
            I: Iterator<Item = &'a str>,
        {
            let mut mark = HashSet::from_iter(entrypoint);

            if mark.is_empty() {
                return mark;
            }

            let mut queue = mark.iter().copied().collect::<VecDeque<_>>();

            while let Some(dep_name) = queue.pop_back() {
                for dep_name in get_dependencies(dep_name) {
                    if mark.insert(dep_name) {
                        queue.push_front(dep_name);
                    }
                }
            }

            mark
        }

        // collect removable packages
        // if the unused package is not referenced by any packages to be removed, it should not be removed
        // since it might be because of bug of VPM implementation
        let removable = {
            // packages to be removed or overridden are entrypoint
            let entrypoint =
                self.package_changes
                    .iter()
                    .filter_map(|(name, change)| match change {
                        PackageChange::Install(change) if change.add_to_locked => {
                            unity_project.get_locked(name.as_str()).map(|x| x.name())
                        }
                        // packages that is not added to locked are not removable
                        PackageChange::Install(_) => None,
                        PackageChange::Remove(_) => {
                            unity_project.get_locked(name.as_str()).map(|x| x.name())
                        }
                    });

            mark_recursive(entrypoint, |dep_name| {
                unity_project
                    .get_locked(dep_name)
                    .into_iter()
                    .flat_map(|dep| dep.dependencies.keys())
                    .map(String::as_str)
            })
        };

        // nothing can be removed
        if removable.is_empty() {
            return;
        }

        // collect packages that is used by dependencies or unlocked packages
        let using_packages = {
            let unlocked_dependencies = unity_project
                .unlocked_packages()
                .iter()
                .filter_map(|(_, pkg)| pkg.as_ref())
                .flat_map(|pkg| pkg.vpm_dependencies().keys())
                .map(String::as_str);

            mark_recursive(
                unlocked_dependencies.chain(unity_project.dependencies()),
                |dep_name| {
                    if let Some(to_install) = self
                        .package_changes
                        .get(dep_name)
                        .and_then(|change| change.as_install())
                        .and_then(|x| x.package)
                    {
                        Either::Left(to_install.vpm_dependencies().keys().map(String::as_str))
                    } else {
                        Either::Right(
                            unity_project
                                .get_locked(dep_name)
                                .into_iter()
                                .flat_map(|dep| dep.dependencies.keys())
                                .map(String::as_str),
                        )
                    }
                },
            )
        };

        // weep
        for locked in unity_project.locked_packages() {
            if !using_packages.contains(locked.name()) && removable.contains(locked.name()) {
                self.remove(locked.name().to_owned(), RemoveReason::Unused);
            }
        }
    }
}

impl<'env> PendingProjectChanges<'env> {
    pub(crate) fn empty() -> Self {
        Self {
            package_changes: Default::default(),
            remove_legacy_files: vec![],
            remove_legacy_folders: vec![],
            conflicts: Default::default(),
        }
    }
}

impl PendingProjectChanges<'_> {
    pub fn package_changes(&self) -> &HashMap<String, PackageChange<'_>> {
        &self.package_changes
    }

    pub fn remove_legacy_files(&self) -> &[PathBuf] {
        self.remove_legacy_files.as_slice()
    }

    pub fn remove_legacy_folders(&self) -> &[PathBuf] {
        self.remove_legacy_folders.as_slice()
    }

    pub fn conflicts(&self) -> &HashMap<String, ConflictInfo> {
        &self.conflicts
    }
}

impl UnityProject {
    /// Applies the changes specified in `AddPackageRequest` to the project.
    pub async fn apply_pending_changes<'env>(
        &mut self,
        env: &'env impl RemotePackageDownloader,
        request: PendingProjectChanges<'env>,
    ) -> io::Result<()> {
        let mut installs = Vec::new();
        let mut remove_names = Vec::new();

        for (name, change) in request.package_changes {
            match change {
                PackageChange::Install(change) => {
                    if let Some(package) = change.package {
                        installs.push(package);
                        if change.add_to_locked {
                            self.manifest.add_locked(
                                package.name(),
                                package.version().clone(),
                                package.vpm_dependencies().clone(),
                            );
                        }
                    }

                    if let Some(version) = change.to_dependencies {
                        self.manifest.add_dependency(&name, version);
                    }
                }
                PackageChange::Remove(_) => {
                    remove_names.push(name);
                }
            }
        }

        self.manifest
            .remove_packages(remove_names.iter().map(String::as_str));

        self.install_packages(env, &installs).await?;

        Self::remove_assets(
            &self.project_dir,
            request.remove_legacy_files.iter().map(PathBuf::as_path),
            request.remove_legacy_folders.iter().map(PathBuf::as_path),
            remove_names.iter().map(String::as_str),
        )
        .await;

        Ok(())
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

    async fn remove_assets(
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
}

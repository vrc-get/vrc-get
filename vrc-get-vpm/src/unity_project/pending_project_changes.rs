use crate::io;
use crate::io::ProjectIo;
use crate::traits::EnvironmentIoHolder;
use crate::unity_project::find_legacy_assets::collect_legacy_assets;
use crate::utils::{copy_recursive, extract_zip};
use crate::version::DependencyRange;
use crate::{
    unity_compatible, PackageInfo, PackageInfoInner, RemotePackageDownloader, UnityProject,
};
use either::Either;
use futures::future::{join3, join_all, try_join_all};
use log::debug;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet, VecDeque};
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
    pub(crate) package_changes: HashMap<Box<str>, PackageChange<'env>>,

    pub(crate) remove_legacy_files: Vec<Box<Path>>,
    pub(crate) remove_legacy_folders: Vec<Box<Path>>,

    pub(crate) conflicts: HashMap<Box<str>, ConflictInfo>,
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

    pub fn as_remove(&self) -> Option<&Remove<'env>> {
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

    pub fn is_adding_to_locked(&self) -> bool {
        self.add_to_locked
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
    conflicts_packages: Vec<Box<str>>,
    conflicts_with_unity: bool,
}

impl ConflictInfo {
    pub fn conflicting_packages(&self) -> &[Box<str>] {
        self.conflicts_packages.as_slice()
    }

    pub fn conflicts_with_unity(&self) -> bool {
        self.conflicts_with_unity
    }
}

pub(crate) struct Builder<'env> {
    package_changes: HashMap<Box<str>, PackageChange<'env>>,
    conflicts: HashMap<Box<str>, ConflictInfo>,
}

impl<'env> Builder<'env> {
    pub fn new() -> Self {
        Self {
            package_changes: HashMap::new(),
            conflicts: HashMap::new(),
        }
    }

    pub fn add_to_dependencies(&mut self, name: Box<str>, version: DependencyRange) -> &mut Self {
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

    pub fn install_to_locked(&mut self, info: PackageInfo<'env>) -> &mut Self {
        match self.package_changes.entry(info.name().into()) {
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

    pub fn install_already_locked(&mut self, info: PackageInfo<'env>) -> &mut Self {
        match self.package_changes.entry(info.name().into()) {
            Entry::Occupied(mut e) => match e.get_mut() {
                PackageChange::Install(e) => {
                    if e.package.is_none() {
                        e.package = Some(info);
                        e.add_to_locked = false;
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
                    add_to_locked: false,
                    to_dependencies: None,
                }));
            }
        }
        self
    }

    pub fn conflict_multiple(
        &mut self,
        name: Box<str>,
        conflict: impl IntoIterator<Item = Box<str>>,
    ) -> &mut Self {
        self.conflicts
            .entry(name)
            .or_default()
            .conflicts_packages
            .extend(conflict);
        self
    }

    pub fn conflicts(&mut self, name: Box<str>, conflict: Box<str>) -> &mut Self {
        self.conflicts
            .entry(name)
            .or_default()
            .conflicts_packages
            .push(conflict);
        self
    }

    pub fn conflicts_unity(&mut self, name: Box<str>) -> &mut Self {
        self.conflicts.entry(name).or_default().conflicts_with_unity = true;
        self
    }

    pub fn remove(&mut self, name: Box<str>, reason: RemoveReason) -> &mut Self {
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

    fn remove_unused(&mut self, name: Box<str>) -> &mut Self {
        match self.package_changes.entry(name) {
            Entry::Occupied(mut e) => match e.get_mut() {
                PackageChange::Install(_) => {
                    panic!("INTERNAL ERROR: remove_unused for installed");
                }
                PackageChange::Remove(_) => {}
            },
            Entry::Vacant(e) => {
                e.insert(PackageChange::Remove(Remove {
                    reason: RemoveReason::Unused,
                    _phantom: PhantomData,
                }));
            }
        }
        self
    }

    pub(crate) fn get_installing(&self, name: &str) -> Option<PackageInfo<'env>> {
        self.package_changes
            .get(name)
            .and_then(|x| x.as_install())
            .filter(|x| x.add_to_locked)
            .and_then(|x| x.package)
    }

    pub(crate) fn get_all_installing(&self) -> impl Iterator<Item = PackageInfo<'env>> + '_ {
        self.package_changes
            .values()
            .filter_map(|x| x.as_install())
            .filter(|x| x.add_to_locked)
            .filter_map(|x| x.package)
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
        unity_project: &UnityProject<impl ProjectIo>,
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
                .map(|pkg| pkg.name().into())
            {
                self.conflicts_unity(package);
            }
        }

        self.mark_and_sweep_packages(unity_project);

        let legacy_assets = collect_legacy_assets(&unity_project.io, &installs).await;

        PendingProjectChanges {
            package_changes: self.package_changes,
            conflicts: self.conflicts,

            remove_legacy_files: legacy_assets.files,
            remove_legacy_folders: legacy_assets.folders,
        }
    }

    fn mark_and_sweep_packages(&mut self, unity_project: &UnityProject<impl ProjectIo>) {
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
                            unity_project.get_locked(name.as_ref()).map(|x| x.name())
                        }
                        // packages that is not added to locked are not removable
                        PackageChange::Install(_) => None,
                        PackageChange::Remove(_) => {
                            unity_project.get_locked(name.as_ref()).map(|x| x.name())
                        }
                    });

            mark_recursive(entrypoint, |dep_name| {
                unity_project
                    .get_locked(dep_name)
                    .into_iter()
                    .flat_map(|dep| dep.dependencies.keys())
                    .map(Box::as_ref)
            })
        };

        debug!("removable packages: {:?}", removable);
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
                .map(Box::as_ref);

            let dependencies = unity_project.dependencies().filter(|name| {
                self.package_changes
                    .get(*name)
                    .and_then(|change| change.as_remove())
                    .is_none()
            });

            mark_recursive(unlocked_dependencies.chain(dependencies), |dep_name| {
                if let Some(to_install) = self
                    .package_changes
                    .get(dep_name)
                    .and_then(|change| change.as_install())
                    .and_then(|x| x.package)
                {
                    Either::Left(to_install.vpm_dependencies().keys().map(Box::as_ref))
                } else {
                    Either::Right(
                        unity_project
                            .get_locked(dep_name)
                            .into_iter()
                            .flat_map(|dep| dep.dependencies.keys())
                            .map(Box::as_ref),
                    )
                }
            })
        };

        debug!("using packages: {:?}", using_packages);

        // weep
        for locked in unity_project.locked_packages() {
            if !using_packages.contains(locked.name()) && removable.contains(locked.name()) {
                self.remove_unused(locked.name().into());
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
    pub fn package_changes(&self) -> &HashMap<Box<str>, PackageChange<'_>> {
        &self.package_changes
    }

    pub fn remove_legacy_files(&self) -> &[Box<Path>] {
        self.remove_legacy_files.as_slice()
    }

    pub fn remove_legacy_folders(&self) -> &[Box<Path>] {
        self.remove_legacy_folders.as_slice()
    }

    pub fn conflicts(&self) -> &HashMap<Box<str>, ConflictInfo> {
        &self.conflicts
    }
}

impl<IO: ProjectIo> UnityProject<IO> {
    /// Applies the changes specified in `AddPackageRequest` to the project.
    pub async fn apply_pending_changes<'env, Env: RemotePackageDownloader + EnvironmentIoHolder>(
        &mut self,
        env: &'env Env,
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
            .remove_packages(remove_names.iter().map(Box::as_ref));

        install_packages(&self.io, env, &installs).await?;

        remove_assets(
            &self.io,
            request.remove_legacy_files.iter().map(Box::as_ref),
            request.remove_legacy_folders.iter().map(Box::as_ref),
            remove_names.iter().map(Box::as_ref),
        )
        .await;

        Ok(())
    }
}

async fn install_packages<Env: RemotePackageDownloader + EnvironmentIoHolder>(
    io: &impl ProjectIo,
    env: &Env,
    packages: &[PackageInfo<'_>],
) -> io::Result<()> {
    // resolve all packages
    try_join_all(
        packages
            .iter()
            .map(|package| add_package(io, env, *package)),
    )
    .await?;

    Ok(())
}

async fn remove_assets(
    io: &impl ProjectIo,
    legacy_files: impl Iterator<Item = &Path>,
    legacy_folders: impl Iterator<Item = &Path>,
    legacy_packages: impl Iterator<Item = &str>,
) {
    join3(
        join_all(legacy_files.map(|relative| async move {
            remove_file(io, relative).await;
        })),
        join_all(legacy_folders.map(|relative| async move {
            remove_folder(io, relative).await;
        })),
        join_all(legacy_packages.map(|name| async move {
            remove_package(io, name).await;
        })),
    )
    .await;

    async fn remove_meta_file(io: &impl ProjectIo, path: PathBuf) {
        let mut building = path.into_os_string();
        building.push(".meta");
        let meta = PathBuf::from(building);

        if let Some(err) = io.remove_file(&meta).await.err() {
            if !matches!(err.kind(), io::ErrorKind::NotFound) {
                log::error!("error removing legacy asset at {}: {}", meta.display(), err);
            }
        }
    }

    async fn remove_file(io: &impl ProjectIo, path: &Path) {
        if let Some(err) = io.remove_file(&path).await.err() {
            log::error!("error removing legacy asset at {}: {}", path.display(), err);
        }
        remove_meta_file(io, path.to_owned()).await;
    }

    async fn remove_folder(io: &impl ProjectIo, path: &Path) {
        if let Some(err) = io.remove_dir_all(&path).await.err() {
            log::error!("error removing legacy asset at {}: {}", path.display(), err);
        }
        remove_meta_file(io, path.to_owned()).await;
    }

    async fn remove_package(io: &impl ProjectIo, name: &str) {
        if let Some(err) = io.remove_dir_all(&format!("Packages/{}", name)).await.err() {
            log::error!("error removing legacy package {}: {}", name, err);
        }
    }
}

pub(crate) async fn add_package<Env: RemotePackageDownloader + EnvironmentIoHolder>(
    io: &impl ProjectIo,
    env: &Env,
    package: PackageInfo<'_>,
) -> io::Result<()> {
    log::debug!("adding package {}", package.name());
    let dest_folder = PathBuf::from(format!("Packages/{}", package.name()));
    match package.inner {
        PackageInfoInner::Remote(package, user_repo) => {
            let zip_file = env.get_package(user_repo, package).await?;

            // remove dest folder before extract if exists
            io.remove_dir_all(&dest_folder).await.ok();
            extract_zip(zip_file, io, &dest_folder).await?;

            Ok(())
        }
        PackageInfoInner::Local(_, path) => {
            io.remove_dir_all(&dest_folder).await.ok();
            // TODO: use io traits
            copy_recursive(env.io(), path.into(), io, dest_folder).await?;
            Ok(())
        }
    }
}

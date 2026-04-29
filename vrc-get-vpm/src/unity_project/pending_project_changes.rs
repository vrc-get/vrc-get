use crate::io::{DefaultProjectIo, IoTrait};
use crate::traits::AbortCheck;
use crate::unity_project::find_legacy_assets::collect_legacy_assets;
use crate::version::DependencyRange;
use crate::{PackageInfo, UnityProject, unity_compatible};
use crate::{PackageInstaller, io};
use either::Either;
use futures::future::{join, join_all};
use futures::prelude::*;
use log::debug;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet, VecDeque};
use std::future::ready;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Represents Packages to be added and folders / packages to be removed
///
/// In vrc-get, Adding package is divided into two phases:
/// - Collect modifications
/// - Apply collected changes
///
/// This is done to ask users before removing packages
#[derive(Debug)]
pub struct PendingProjectChanges<'env> {
    pub(crate) package_changes: HashMap<Box<str>, PackageChange<'env>>,

    pub(crate) remove_legacy_files: Vec<(Box<Path>, &'env str)>,
    pub(crate) remove_legacy_folders: Vec<(Box<Path>, &'env str)>,

    pub(crate) conflicts: HashMap<Box<str>, ConflictInfo>,
}

#[derive(Debug)]
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

#[derive(Debug)]
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

    pub fn to_dependencies(&self) -> Option<&DependencyRange> {
        self.to_dependencies.as_ref()
    }
}

#[derive(Debug)]
pub struct Remove<'env> {
    reason: RemoveReason,
    _phantom: PhantomData<&'env ()>,
}

impl Remove<'_> {
    pub fn reason(&self) -> RemoveReason {
        self.reason
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RemoveReason {
    Requested,
    Legacy,
    Unused,
}

#[derive(Debug, Default)]
pub struct ConflictInfo {
    conflicts_packages: Vec<Box<str>>,
    conflicts_with_unity: bool,
    // The value is the name of directory that is installed as unlocked
    unlocked_names: Vec<Box<str>>,
}

impl ConflictInfo {
    pub fn conflicting_packages(&self) -> &[Box<str>] {
        self.conflicts_packages.as_slice()
    }

    pub fn conflicts_with_unity(&self) -> bool {
        self.conflicts_with_unity
    }

    pub fn unlocked_names(&self) -> &[Box<str>] {
        self.unlocked_names.as_slice()
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
        match self.package_changes.entry(name.clone()) {
            Entry::Occupied(mut e) => match e.get_mut() {
                PackageChange::Install(e) => {
                    if e.to_dependencies.is_none() {
                        e.to_dependencies = Some(version);
                    } else {
                        panic!("INTERNAL ERROR: already add_to_dependencies: {}", name);
                    }
                }
                PackageChange::Remove(_) => {
                    panic!("INTERNAL ERROR: add_to_dependencies for removed: {}", name);
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
        self.install_to_locked_impl(info, true, false)
    }

    // replacing is necessary for resolve since dependencies or unlocked package may require
    // newer versions of locked packages.
    pub fn install_to_locked_replacing(&mut self, info: PackageInfo<'env>) -> &mut Self {
        self.install_to_locked_impl(info, true, true)
    }

    pub fn install_already_locked(&mut self, info: PackageInfo<'env>) -> &mut Self {
        self.install_to_locked_impl(info, false, false)
    }

    fn install_to_locked_impl(
        &mut self,
        info: PackageInfo<'env>,
        add_to_locked: bool,
        allow_replace: bool,
    ) -> &mut Self {
        match self.package_changes.entry(info.name().into()) {
            Entry::Occupied(mut e) => match e.get_mut() {
                PackageChange::Install(e) => {
                    if let Some(installed) = e.package
                        && !allow_replace
                    {
                        panic!(
                            "INTERNAL ERROR: already install: {} ({} ({}) => {} ({}))",
                            info.name(),
                            installed.version(),
                            if e.add_to_locked {
                                "to locked"
                            } else {
                                "existing"
                            },
                            info.version(),
                            if add_to_locked {
                                "to locked"
                            } else {
                                "existing"
                            },
                        );
                    } else {
                        e.package = Some(info);
                        e.add_to_locked = add_to_locked;
                    }
                }
                PackageChange::Remove(_) => {
                    panic!("INTERNAL ERROR: install for removed: {}", info.name());
                }
            },
            Entry::Vacant(e) => {
                e.insert(PackageChange::Install(Install {
                    package: Some(info),
                    add_to_locked,
                    to_dependencies: None,
                }));
            }
        }
        self
    }

    pub fn unlocked_installation_conflict(
        &mut self,
        name: Box<str>,
        unlocked_name: Box<str>,
    ) -> &mut Self {
        self.conflicts
            .entry(name)
            .or_default()
            .unlocked_names
            .push(unlocked_name);
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
        match self.package_changes.entry(name.clone()) {
            Entry::Occupied(mut e) => match e.get_mut() {
                PackageChange::Install(_) => {
                    panic!("INTERNAL ERROR: remove for installed: {name}");
                }
                PackageChange::Remove(e) => {
                    if e.reason != reason {
                        panic!("INTERNAL ERROR: already remove: {name}");
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
        match self.package_changes.entry(name.clone()) {
            Entry::Occupied(mut e) => match e.get_mut() {
                PackageChange::Install(_) => {
                    panic!("INTERNAL ERROR: remove_unused for installed: {name}");
                }
                PackageChange::Remove(_) => {
                    // already removed, do nothing
                }
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

    pub(crate) fn get_dependencies(&self, name: &str) -> Option<&DependencyRange> {
        self.package_changes
            .get(name)
            .and_then(|x| x.as_install())
            .and_then(|x| x.to_dependencies.as_ref())
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
        unity_project: &UnityProject,
    ) -> PendingProjectChanges<'env> {
        let installs = Vec::from_iter(
            self.package_changes
                .values()
                .filter_map(|x| x.as_install())
                .filter(|x| x.add_to_locked)
                .map(|x| x.package.unwrap()),
        );

        debug!("checking for unity compatibility");

        for package in installs
            .iter()
            .filter(|pkg| !unity_compatible(pkg.package_json(), unity_project.unity_version))
            .map(|pkg| pkg.name().into())
        {
            self.conflicts_unity(package);
        }

        debug!("Finding unused packages");

        self.mark_and_sweep_packages(unity_project);

        debug!("Collecting legacy assets");

        let legacy_assets =
            collect_legacy_assets(&unity_project.io, &installs, unity_project).await;

        debug!("Building PendingProjectChanges finished!");

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

        debug!("removable packages: {removable:?}");
        // nothing can be removed
        if removable.is_empty() {
            return;
        }

        // copy to avoid borrow checker error
        let installing_packages = self
            .package_changes
            .iter()
            .filter(|(_, change)| change.as_install().is_some())
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();

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

            // keep installing packages even if the package is not used by any dependencies
            let package_changes = installing_packages.iter().map(Box::as_ref);

            let entry_points = unlocked_dependencies
                .chain(dependencies)
                .chain(package_changes);

            mark_recursive(entry_points, |dep_name| {
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

        debug!("using packages: {using_packages:?}");

        // weep
        for locked in unity_project.locked_packages() {
            if !using_packages.contains(locked.name()) && removable.contains(locked.name()) {
                self.remove_unused(locked.name().into());
            }
        }
    }
}

impl PendingProjectChanges<'_> {
    pub fn package_changes(&self) -> &HashMap<Box<str>, PackageChange<'_>> {
        &self.package_changes
    }

    pub fn remove_legacy_files(&self) -> &[(Box<Path>, &str)] {
        self.remove_legacy_files.as_slice()
    }

    pub fn remove_legacy_folders(&self) -> &[(Box<Path>, &str)] {
        self.remove_legacy_folders.as_slice()
    }

    pub fn conflicts(&self) -> &HashMap<Box<str>, ConflictInfo> {
        &self.conflicts
    }
}

static TEMP_DIR: &str = "Temp";
static PKG_TEMP_DIR: &str = "Temp/vrc-get";

impl UnityProject {
    /// Applies the changes specified in `AddPackageRequest` to the project.
    ///
    /// This will also save the manifest changes
    pub async fn apply_pending_changes<'env, Env: PackageInstaller>(
        &mut self,
        env: &'env Env,
        request: PendingProjectChanges<'env>,
    ) -> io::Result<()> {
        /*
        Apply pending changes consists of following steps:
        - Extract new packages to temp directory
        - Actually install packages
          - Move old packages to temp directory (remove packages)
          - Move new packages to Package directory (add packages)
          - Apply changes to manifest
        - Remove legacy assets
        - Cleanup temp directory as possible (errors ignored)
         */

        let mut installs = Vec::new();
        let mut remove_names = Vec::new();
        let mut uninstall_packages = Vec::new();
        let mut remove_unlocked_names = Vec::new();

        for (name, change) in &request.package_changes {
            match change {
                PackageChange::Install(change) => {
                    if let Some(package) = change.package {
                        installs.push(package);
                        uninstall_packages.push(name.as_ref());
                    }
                }
                PackageChange::Remove(_) => {
                    remove_names.push(name.as_ref());
                    uninstall_packages.push(name.as_ref());
                }
            }
        }

        for info in request.conflicts.values() {
            for x in &info.unlocked_names {
                remove_unlocked_names.push(x.as_ref());
                uninstall_packages.push(x.as_ref());
            }
        }

        let temp_dir_base = PathBuf::from(format!("{PKG_TEMP_DIR}/{}", uuid::Uuid::new_v4()));
        let mut context = InstallPackageContext::new(&temp_dir_base);

        // Before all, prepare temp dir
        self.io.create_dir_all(&context.remove_temp_dir).await?;
        self.io.create_dir_all(&context.install_temp_dir).await?;

        let mut r: io::Result<()> = async {
            // Firstly, extract packages
            extract_packages(&self.io, env, &context.install_temp_dir, &installs).await?;

            // Then, update packages directory
            context
                .move_uninstall_packages(&self.io, &uninstall_packages)
                .await?;
            context.move_install_packages(&self.io, &installs).await?;

            // apply changes to manifest
            for (name, change) in &request.package_changes {
                if let PackageChange::Install(change) = change {
                    if let Some(package) = change.package
                        && change.add_to_locked
                    {
                        self.manifest.add_locked(
                            package.name(),
                            package.version().clone(),
                            package.vpm_dependencies().clone(),
                        );
                    }

                    if let Some(version) = &change.to_dependencies {
                        self.manifest.add_dependency(name, version.clone());
                    }
                }
            }
            self.manifest.remove_packages(remove_names.iter().copied());
            self.save().await?;
            Ok(())
        }
        .await;

        if let Err(mut e) = r {
            // When error occurs in installation process, we try rolling back package changes.
            // Since project manifest file change is the last operation of the project change,
            // we don't need to rollback manifest changes.
            if let Err(rollback_error) = context.rollback_changes(&self.io).await {
                // Rolling back packages failed. this is very unlikely but this state is very
                // fatal and project is in completely broken state so we provide much information
                // as possible though error mesasge.
                #[derive(Debug)]
                struct RollingBackError {
                    original_error: io::Error,
                    rollback_error: io::Error,
                    remaining_installed_packages: Vec<Box<str>>,
                    remaining_removed_packages: Vec<Box<str>>,
                }

                impl std::error::Error for RollingBackError {}

                impl std::fmt::Display for RollingBackError {
                    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        f.write_fmt(format_args!(
                            "Error rolling back packages: {rollback_error}\n\
                                original error:{original_error}\n\
                                falsy installed packages: {installed_packages:?}\n\
                                falsy removed packages: {removed_packages:?}",
                            rollback_error = self.rollback_error,
                            original_error = self.original_error,
                            installed_packages = self.remaining_installed_packages,
                            removed_packages = self.remaining_removed_packages,
                        ))
                    }
                }

                e = io::Error::new(
                    e.kind(),
                    RollingBackError {
                        original_error: e,
                        rollback_error,
                        remaining_installed_packages: (context.installed.into_iter())
                            .map(Box::from)
                            .collect(),
                        remaining_removed_packages: (context.removed.into_iter())
                            .map(Box::from)
                            .collect(),
                    },
                );

                // If user is familiar with vrc-get internals, user might recover packages
                // from temp directory so we don't clean temp directory.
            } else {
                // cleanup temp directory when rollback successfully finished
                cleanup_temp_dir(&self.io).await;
            }

            r = Err(e);
        } else {
            // installation process finished successfully

            // cleanup
            cleanup_temp_dir(&self.io).await;

            // remove legacy assets
            remove_assets(
                &self.io,
                request.remove_legacy_files.iter().map(|(p, _)| p.as_ref()),
                request
                    .remove_legacy_folders
                    .iter()
                    .map(|(p, _)| p.as_ref()),
            )
            .await;
        }

        async fn cleanup_temp_dir(io: &DefaultProjectIo) {
            // cleanup temp directory
            // we ignore error since some file might be locked by unity.
            io.remove_dir_all(PKG_TEMP_DIR.as_ref()).await.ok();
            // remove temp dir also if it's empty
            io.remove_dir(TEMP_DIR.as_ref()).await.ok();
        }

        r
    }
}

async fn extract_packages<Env: PackageInstaller>(
    io: &DefaultProjectIo,
    env: &Env,
    extract_dir: &Path,
    packages: &[PackageInfo<'_>],
) -> io::Result<()> {
    let abort = AbortCheck::new();
    let mut error_store = OnceLock::new();

    // resolve all packages
    join_all(packages.iter().map(|package| {
        async {
            env.install_package(io, *package, &extract_dir.join(package.name()), &abort)
                .await
        }
        .then(|x| {
            if let Err(e) = x {
                error_store.set(e).ok();
                abort.abort();
            }
            ready(())
        })
    }))
    .await;

    if let Some(err) = error_store.take() {
        return Err(err);
    }

    Ok(())
}

/// The context that holds information to restore changes to packages directory when recoverable error occurs
struct InstallPackageContext<'a> {
    removed: HashSet<&'a str>,
    installed: HashSet<&'a str>,

    remove_temp_dir: PathBuf,
    install_temp_dir: PathBuf,
}

impl<'a> InstallPackageContext<'a> {
    fn new(temp_dir_base: &Path) -> Self {
        Self {
            removed: HashSet::new(),
            installed: HashSet::new(),

            remove_temp_dir: temp_dir_base.join("remove"),
            install_temp_dir: temp_dir_base.join("install"),
        }
    }

    async fn move_uninstall_packages(
        &mut self,
        io: &DefaultProjectIo,
        packages: &[&'a str],
    ) -> io::Result<()> {
        // it's expected to cheap to rename (link) packages to temp dir,
        // so we do it sequentially for simplicity
        let packages_dir = Path::new("Packages");

        for package in packages {
            if self.removed.contains(package) {
                continue;
            }

            let package_dir = packages_dir.join(package);
            let copied_dir = self.remove_temp_dir.join(package);

            match io.rename(&package_dir, &copied_dir).await {
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                    // the package does not exist, ignore this
                }
                Err(e) => return Err(e),
                Ok(()) => {
                    self.removed.insert(package);
                }
            }
        }

        Ok(())
    }

    async fn move_install_packages(
        &mut self,
        io: &DefaultProjectIo,
        packages: &[PackageInfo<'a>],
    ) -> io::Result<()> {
        let packages_dir = Path::new("Packages");

        for package in packages.iter() {
            io.rename(
                &self.install_temp_dir.join(package.name()),
                &packages_dir.join(package.name()),
            )
            .await?;
            self.installed.insert(package.name());
        }

        Ok(())
    }

    async fn rollback_changes(&mut self, io: &DefaultProjectIo) -> io::Result<()> {
        let packages_dir = Path::new("Packages");

        for installed in self.installed.drain() {
            io.rename(
                &packages_dir.join(installed),
                &self.install_temp_dir.join(installed),
            )
            .await?;
        }

        for removed in self.removed.drain() {
            io.rename(
                &self.remove_temp_dir.join(removed),
                &packages_dir.join(removed),
            )
            .await?;
        }

        Ok(())
    }
}

async fn remove_assets(
    io: &DefaultProjectIo,
    legacy_files: impl Iterator<Item = &Path>,
    legacy_folders: impl Iterator<Item = &Path>,
) {
    join(
        join_all(legacy_files.map(|relative| async move {
            remove_file(io, relative).await;
        })),
        join_all(legacy_folders.map(|relative| async move {
            remove_folder(io, relative).await;
        })),
    )
    .await;

    async fn remove_meta_file(io: &DefaultProjectIo, path: PathBuf) {
        let mut building = path.into_os_string();
        building.push(".meta");
        let meta = PathBuf::from(building);

        if let Some(err) = io.remove_file(&meta).await.err()
            && !matches!(err.kind(), io::ErrorKind::NotFound)
        {
            log::error!("error removing legacy asset at {}: {err}", meta.display());
        }
    }

    async fn remove_file(io: &DefaultProjectIo, path: &Path) {
        if let Some(err) = io.remove_file(path).await.err() {
            log::error!("error removing legacy asset at {}: {err}", path.display());
        }
        remove_meta_file(io, path.to_owned()).await;
    }

    async fn remove_folder(io: &DefaultProjectIo, path: &Path) {
        if let Some(err) = io.remove_dir_all(path).await.err() {
            log::error!("error removing legacy asset at {}: {err}", path.display());
        }
        remove_meta_file(io, path.to_owned()).await;
    }
}

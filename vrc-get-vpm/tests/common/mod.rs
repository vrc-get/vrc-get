// since this moduke is shared between multiple tests, we need to allow dead code and unused imports
#![allow(dead_code)]
#![allow(unused_imports)]

mod package_collection;
mod virtual_environment;
mod virtual_project_builder;

pub use package_collection::PackageCollection;
pub use package_collection::PackageCollectionBuilder;
use std::path::{Path, PathBuf};
pub use virtual_environment::VirtualInstaller;
pub use virtual_project_builder::VirtualProjectBuilder;

use vrc_get_vpm::PackageInfo;
use vrc_get_vpm::unity_project::PendingProjectChanges;
use vrc_get_vpm::unity_project::pending_project_changes::RemoveReason;
use vrc_get_vpm::version::{DependencyRange, Version};

pub fn assert_removed(result: &PendingProjectChanges, package: &str, reason: RemoveReason) {
    let package_change = result
        .package_changes()
        .get(package)
        .expect("the package is not changed");
    let package_change = package_change.as_remove().expect("package is not removing");
    assert_eq!(package_change.reason(), reason);
}

pub fn assert_install_only(result: &PendingProjectChanges, package: &PackageInfo) {
    let change = result
        .package_changes()
        .get(package.name())
        .expect("the package is not changed");
    let install = change.as_install().expect("the package is not installing");
    assert!(!install.is_adding_to_locked());
    assert!(install.to_dependencies().is_none());
    let installing = install.install_package().expect("not installing");
    assert_eq!(
        installing.package_json() as *const _,
        package.package_json() as *const _
    );
}

pub fn assert_installing_to_locked_only(result: &PendingProjectChanges, package: &PackageInfo) {
    let change = result
        .package_changes()
        .get(package.name())
        .expect("the package is not changed");
    let install = change.as_install().expect("the package is not installing");
    assert!(install.is_adding_to_locked());
    assert!(install.to_dependencies().is_none());
    let installing = install.install_package().expect("not installing");
    assert_eq!(
        installing.package_json() as *const _,
        package.package_json() as *const _
    );
}

pub fn assert_installing_to_both(result: &PendingProjectChanges, package: &PackageInfo) {
    let change = result
        .package_changes()
        .get(package.name())
        .expect("the package is not changed");
    let install = change.as_install().expect("package is not installing");
    assert!(install.is_adding_to_locked());
    let range = install.to_dependencies().unwrap();
    assert_eq!(range, &DependencyRange::version(package.version().clone()));
    let installing = install.install_package().expect("no package");
    assert_eq!(
        installing.package_json() as *const _,
        package.package_json() as *const _
    );
}

pub fn assert_installing_to_dependencies_only(
    result: &PendingProjectChanges,
    name: &str,
    version: Version,
) {
    let base_change = result
        .package_changes()
        .get(name)
        .expect("the package is not changed");
    let base_change = base_change
        .as_install()
        .expect("the package is not installing");
    assert!(!base_change.is_adding_to_locked(), "is adding to locked");
    assert!(
        base_change.install_package().is_none(),
        "installing package"
    );
    let base_range = base_change
        .to_dependencies()
        .expect("not installing to dependencies");
    assert_eq!(base_range, &DependencyRange::version(version));
}

pub fn block_on<F: Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_multi_thread()
        .build()
        .unwrap()
        .block_on(f)
}

#[track_caller]
pub fn get_temp_path(base_name: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "{base_name}/{}_L{}",
        env!("CARGO_CRATE_NAME"),
        std::panic::Location::caller().line()
    ))
}

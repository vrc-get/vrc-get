/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

// since this moduke is shared between multiple tests, we need to allow dead code and unused imports
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_macros)]

mod package_collection;
mod virtual_environment;
mod virtual_project_builder;

pub use package_collection::PackageCollection;
pub use package_collection::PackageCollectionBuilder;
use std::fmt::Display;
use std::path::{Path, PathBuf};
pub use virtual_environment::VirtualInstaller;
pub use virtual_project_builder::VirtualProjectBuilder;

use vrc_get_vpm::PackageInfo;
use vrc_get_vpm::unity_project::PendingProjectChanges;
use vrc_get_vpm::unity_project::pending_project_changes::{Install, RemoveReason};
use vrc_get_vpm::version::{DependencyRange, Version};

/// Asserts the number of elements of a collection in the changes, like `result.package_changes()`.
///
/// Unlike `assert_eq!` with `len()`, this shows what the changes are when the assertion fails.
macro_rules! assert_len {
    ($result:ident.$collection:ident(), $expected:expr $(,)?) => {{
        let actual: usize = $result.$collection().len();
        let expected: usize = $expected;
        assert!(
            actual == expected,
            "`{}` has {} elements but {} expected\n{}",
            concat!(stringify!($result), ".", stringify!($collection), "()"),
            actual,
            expected,
            $crate::common::describe_changes(&$result),
        );
    }};
}
pub(crate) use assert_len;

/// Describes the changes shortly to show them in the messages of assertions.
pub fn describe_changes(result: &PendingProjectChanges) -> impl Display {
    std::fmt::from_fn(move |f| {
        let mut package_changes = result.package_changes().iter().collect::<Vec<_>>();
        package_changes.sort_by_key(|&(name, _)| name);
        writeln!(f, "package changes ({}):", package_changes.len())?;
        for (name, change) in package_changes {
            let mut actions = Vec::new();
            if let Some(install) = change.as_install() {
                if let Some(package) = install.install_package() {
                    actions.push(format!("install {}", package.version()));
                }
                if install.is_adding_to_locked() {
                    actions.push("add to locked".to_string());
                }
                if let Some(range) = install.to_dependencies() {
                    actions.push(format!("add to dependencies as {range}"));
                }
            }
            if let Some(remove) = change.as_remove() {
                actions.push(format!("remove as {:?}", remove.reason()));
            }
            if actions.is_empty() {
                actions.push("(no-op install)".to_string());
            }
            writeln!(f, "  {name}: {}", actions.join(", "))?;
        }

        let legacy_assets = [
            ("remove legacy folders", result.remove_legacy_folders()),
            ("remove legacy files", result.remove_legacy_files()),
        ];
        for (kind, assets) in legacy_assets {
            writeln!(f, "{kind} ({}):", assets.len())?;
            for (path, package) in assets {
                writeln!(f, "  {} (of {package})", path.display())?;
            }
        }

        let mut conflicts = result.conflicts().iter().collect::<Vec<_>>();
        conflicts.sort_by_key(|&(name, _)| name);
        writeln!(f, "conflicts ({}):", conflicts.len())?;
        for (name, conflict) in conflicts {
            let mut reasons = Vec::new();
            if !conflict.conflicting_packages().is_empty() {
                reasons.push(format!(
                    "with packages {:?}",
                    conflict.conflicting_packages()
                ));
            }
            if conflict.conflicts_with_unity() {
                reasons.push("with unity".to_string());
            }
            if !conflict.unlocked_names().is_empty() {
                reasons.push(format!("with unlocked {:?}", conflict.unlocked_names()));
            }
            if reasons.is_empty() {
                reasons.push("(no reason)".to_string());
            }
            writeln!(f, "  {name}: {}", reasons.join(", "))?;
        }

        Ok(())
    })
}

/// Fails the test with the message and what the changes are.
macro_rules! fail {
    ($result:expr, $($message:tt)+) => {
        panic!("{}\n{}", format_args!($($message)+), describe_changes($result))
    };
}

#[track_caller]
pub fn assert_removed(result: &PendingProjectChanges, package: &str, reason: RemoveReason) {
    let Some(change) = result.package_changes().get(package) else {
        fail!(result, "{package} is not changed");
    };
    let Some(remove) = change.as_remove() else {
        fail!(result, "{package} is not removing");
    };
    if remove.reason() != reason {
        fail!(result, "{package} is not removing as {reason:?}");
    }
}

#[track_caller]
pub fn assert_install_only(result: &PendingProjectChanges, package: &PackageInfo) {
    let name = package.name();
    let install = find_install(result, name);
    if install.is_adding_to_locked() {
        fail!(result, "{name} is adding to locked");
    }
    if install.to_dependencies().is_some() {
        fail!(result, "{name} is adding to dependencies");
    }
    assert_installing_package(result, install, package);
}

#[track_caller]
pub fn assert_installing_to_locked_only(result: &PendingProjectChanges, package: &PackageInfo) {
    let name = package.name();
    let install = find_install(result, name);
    if !install.is_adding_to_locked() {
        fail!(result, "{name} is not adding to locked");
    }
    if install.to_dependencies().is_some() {
        fail!(result, "{name} is adding to dependencies");
    }
    assert_installing_package(result, install, package);
}

#[track_caller]
pub fn assert_installing_to_both(result: &PendingProjectChanges, package: &PackageInfo) {
    let name = package.name();
    let install = find_install(result, name);
    if !install.is_adding_to_locked() {
        fail!(result, "{name} is not adding to locked");
    }
    assert_adding_to_dependencies(result, install, name, package.version());
    assert_installing_package(result, install, package);
}

#[track_caller]
pub fn assert_installing_to_dependencies_only(
    result: &PendingProjectChanges,
    name: &str,
    version: Version,
) {
    let install = find_install(result, name);
    if install.is_adding_to_locked() {
        fail!(result, "{name} is adding to locked");
    }
    if install.install_package().is_some() {
        fail!(result, "{name} is installing a package");
    }
    assert_adding_to_dependencies(result, install, name, &version);
}

/// Finds the installation of the package in the changes.
#[track_caller]
fn find_install<'a>(result: &'a PendingProjectChanges, name: &str) -> &'a Install<'a> {
    let Some(change) = result.package_changes().get(name) else {
        fail!(result, "{name} is not changed");
    };
    let Some(install) = change.as_install() else {
        fail!(result, "{name} is not installing");
    };
    install
}

/// Asserts the installation installs the `package` itself, not only the same version.
#[track_caller]
fn assert_installing_package(
    result: &PendingProjectChanges,
    install: &Install,
    package: &PackageInfo,
) {
    let name = package.name();
    let version = package.version();
    let Some(installing) = install.install_package() else {
        fail!(result, "{name} is not installing a package");
    };
    if !std::ptr::eq(installing.package_json(), package.package_json()) {
        fail!(result, "{name} is not installing the package of {version}");
    }
}

/// Asserts the installation adds the package to dependencies as the `version`.
#[track_caller]
fn assert_adding_to_dependencies(
    result: &PendingProjectChanges,
    install: &Install,
    name: &str,
    version: &Version,
) {
    let Some(range) = install.to_dependencies() else {
        fail!(result, "{name} is not adding to dependencies");
    };
    if range != &DependencyRange::version(version.clone()) {
        fail!(result, "{name} is not adding to dependencies as {version}");
    }
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

pub fn init_log() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Trace)
        .is_test(true)
        .try_init()
        .ok();
}

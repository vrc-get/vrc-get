/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

//! Property based tests for the package resolution.
//!
//! We generate small random repositories, installed packages, and requests,
//! and check the resolution terminates and the result is consistent.
//!
//! Only which versions a range accepts matters, so we use a few versions and ranges.

use common::*;
use proptest::prelude::*;
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;
use vrc_get_vpm::unity_project::AddPackageErr;
use vrc_get_vpm::unity_project::{AddPackageOperation, PendingProjectChanges};
use vrc_get_vpm::version::{PrereleaseAcceptance, Version, VersionRange};
use vrc_get_vpm::{PackageInfo, PackageManifest};

mod common;

const NAMES: &[&str] = &["com.test.a", "com.test.b", "com.test.c"];
// Sorted from newer to older since the collection for tests returns the first match.
const VERSIONS: &[&str] = &["2.0.0", "2.0.0-beta", "1.1.0", "1.0.0"];
// Stable packages accept the prerelease version only with "^2.0.0-beta",
// unless it's the only way to use a version compatible with the unity.
const RANGES: &[&str] = &[
    ">=1.0.0",
    "^1.0.0",
    "^2.0.0",
    ">=1.1.0",
    "<1.1.0",
    "~1.1.0",
    "^2.0.0-beta",
];
// The project is for unity 2019 by default, so the packages for 2022.3 are incompatible.
const NEW_UNITY: &str = "2022.3";

const TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
struct PackageSpec {
    name: usize,
    version: usize,
    // (name, range)
    dependencies: Vec<(usize, usize)>,
    new_unity: bool,
}

#[derive(Debug, Clone)]
struct Case {
    // Sorted from newer to older since the collection for tests returns the first match.
    packages: Vec<PackageSpec>,
    // (name, version, in dependencies)
    installed: Vec<(usize, usize, bool)>,
    // (name, version)
    requests: Vec<(usize, usize)>,
}

fn case() -> impl Strategy<Value = Case> {
    // Most versions exist and depend on most other packages, so versions are often replaced,
    // which is where the resolution gets complicated.
    let version = (
        prop::bool::weighted(0.85),
        prop::collection::vec(prop::option::weighted(0.7, 0..RANGES.len()), NAMES.len()),
        prop::bool::weighted(0.2),
    );
    (
        prop::collection::vec(version, NAMES.len() * VERSIONS.len()),
        prop::collection::vec((any::<prop::sample::Index>(), any::<bool>()), 0..=3),
        prop::collection::vec(any::<prop::sample::Index>(), 1..=2),
    )
        .prop_filter_map("no packages", |(versions, installed, requests)| {
            let packages = (versions.into_iter().enumerate())
                .filter(|(_, (exists, _, _))| *exists)
                .map(|(index, (_, dependencies, new_unity))| {
                    let name = index / VERSIONS.len();
                    let version = index % VERSIONS.len();
                    PackageSpec {
                        name,
                        version,
                        // a package never depends on itself
                        dependencies: (dependencies.into_iter().enumerate())
                            .filter(|&(dependency, _)| dependency != name)
                            .filter_map(|(dependency, range)| Some((dependency, range?)))
                            .collect(),
                        new_unity,
                    }
                })
                .collect::<Vec<_>>();
            if packages.is_empty() {
                return None;
            }

            let get = |index: prop::sample::Index| {
                let package = index.get(&packages);
                (package.name, package.version)
            };

            // only existing versions can be installed or requested, one version for each package
            let mut installed = (installed.into_iter())
                .map(|(index, in_dependencies)| {
                    let (name, version) = get(index);
                    (name, version, in_dependencies)
                })
                .collect::<Vec<_>>();
            installed.sort_by_key(|x| x.0);
            installed.dedup_by_key(|x| x.0);

            let mut requests = requests.into_iter().map(get).collect::<Vec<_>>();
            requests.sort_by_key(|x| x.0);
            requests.dedup_by_key(|x| x.0);

            Some(Case {
                packages,
                installed,
                requests,
            })
        })
}

fn manifest(spec: &PackageSpec) -> PackageManifest {
    let dependencies = (spec.dependencies.iter())
        .map(|&(name, range)| (NAMES[name].into(), RANGES[range].into()))
        .collect::<serde_json::Map<_, _>>();
    let mut json = serde_json::json!({
        "name": NAMES[spec.name],
        "version": VERSIONS[spec.version],
        "vpmDependencies": dependencies,
    });
    if spec.new_unity {
        json["unity"] = NEW_UNITY.into();
    }
    serde_json::from_value(json).unwrap()
}

fn version(index: usize) -> Version {
    VERSIONS[index].parse().unwrap()
}

/// Runs the resolution for the case, and checks the result.
///
/// The resolution runs on another thread to fail instead of hanging if it never terminates.
fn run(case: Case) -> Result<(), TestCaseError> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = check(&case);
        sender.send(result).ok();
    });
    match receiver.recv_timeout(TIMEOUT) {
        Ok(result) => result.map_err(TestCaseError::fail),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err(TestCaseError::fail("the resolution did not terminate"))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(TestCaseError::fail("the resolution panicked"))
        }
    }
}

fn check(case: &Case) -> Result<(), String> {
    let mut collection = PackageCollectionBuilder::new();
    for spec in &case.packages {
        collection = collection.add(manifest(spec));
    }
    let collection = collection.build();

    let mut project = VirtualProjectBuilder::new();
    for &(name, version_index, in_dependencies) in &case.installed {
        let package = collection.get_package(NAMES[name], version(version_index));
        let dependencies = (package.vpm_dependencies().iter())
            .map(|(name, range)| (name.as_ref(), range.to_string()))
            .collect::<Vec<_>>();
        let dependencies = (dependencies.iter())
            .map(|(name, range)| (*name, range.as_str()))
            .collect::<Vec<_>>();
        project.add_locked(NAMES[name], version(version_index), &dependencies);
        if in_dependencies {
            project.add_dependency(NAMES[name], version(version_index));
        }
    }

    let requests = (case.requests.iter())
        .map(|&(name, version_index)| collection.get_package(NAMES[name], version(version_index)))
        .collect::<Vec<_>>();

    block_on(async {
        let project = project.build().await.unwrap();

        let result = project
            .add_package_request(
                &collection,
                &requests,
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await;

        let result = match result {
            Ok(result) => result,
            // No version of some dependency exists.
            Err(AddPackageErr::DependenciesNotFound { .. }) => return Ok(()),
            Err(e) => return Err(format!("unexpected error: {e}")),
        };

        check_result(case, &requests, &result)
    })
}

fn check_result(
    case: &Case,
    requests: &[PackageInfo],
    result: &PendingProjectChanges,
) -> Result<(), String> {
    let describe = || describe_changes(result);

    // The packages installed after applying the changes.
    let mut installed = HashMap::<&str, Version>::new();
    for &(name, version_index, _) in &case.installed {
        installed.insert(NAMES[name], version(version_index));
    }
    for (name, change) in result.package_changes() {
        if let Some(install) = change.as_install() {
            if let Some(package) = install.install_package() {
                installed.insert(name, package.version().clone());
            }
        } else {
            installed.remove(name.as_ref());
        }
    }
    let changed = |name: &str| {
        (result.package_changes().get(name)).is_some_and(|change| {
            change.as_remove().is_some()
                || (change.as_install()).is_some_and(|x| x.install_package().is_some())
        })
    };
    let dependencies_of = |name: &str| -> &[(usize, usize)] {
        let version = &installed[name];
        let spec = (case.packages.iter())
            .find(|x| NAMES[x.name] == name && &self::version(x.version) == version)
            .unwrap();
        &spec.dependencies
    };
    let range = |index: usize| -> VersionRange { RANGES[index].parse().unwrap() };
    // The installed prerelease versions are selected or accepted by someone,
    // so they never conflict with ranges containing them, like the resolution does.
    let satisfies = |range_index: usize, version: &Version| {
        range(range_index).match_pre(version, PrereleaseAcceptance::Allow)
    };

    // The requested versions are installed.
    // If the newer version is installed, we keep it and only add the package to dependencies.
    for request in requests {
        match installed.get(request.name()) {
            Some(version) if version >= request.version() => {}
            _ => {
                return Err(format!(
                    "requested {} {} is not installed\n{}",
                    request.name(),
                    request.version(),
                    describe()
                ));
            }
        }
    }

    // The requirements related to the changes are satisfied, or reported as conflicts.
    for &name in installed.keys() {
        for &(dependency, range_index) in dependencies_of(name) {
            let dependency = NAMES[dependency];
            if !changed(name) && !changed(dependency) {
                // We never check the requirements we don't touch.
                continue;
            }
            let Some(dependency_version) = installed.get(dependency) else {
                return Err(format!(
                    "{dependency} required by {name} is not installed\n{}",
                    describe()
                ));
            };
            let reported = (result.conflicts().get(dependency))
                .is_some_and(|x| x.conflicting_packages().iter().any(|x| x.as_ref() == name));
            if !satisfies(range_index, dependency_version) && !reported {
                return Err(format!(
                    "{dependency} {dependency_version} doesn't satisfy {} required by {name}, \
                    but not reported as a conflict\n{}",
                    RANGES[range_index],
                    describe()
                ));
            }
        }
    }

    // The reported conflicts are real.
    for (dependency, conflict) in result.conflicts() {
        // We report the existing conflicts only with new conflicts of the same dependency,
        // and never re-report the existing conflicts the changes are unrelated to.
        let has_new_conflict = changed(dependency)
            || (conflict.conflicting_packages().iter()).any(|name| changed(name));
        if !has_new_conflict {
            return Err(format!(
                "{dependency} is reported as conflict but has no new conflict\n{}",
                describe()
            ));
        }
        for name in conflict.conflicting_packages() {
            if name.is_empty() {
                // The requirement from the project
                continue;
            }
            let Some(dependency_version) = installed.get(dependency.as_ref()) else {
                return Err(format!(
                    "{dependency} is reported as conflict but not installed\n{}",
                    describe()
                ));
            };
            if !installed.contains_key(name.as_ref()) {
                return Err(format!(
                    "{name} is reported as conflicting with {dependency} but not installed\n{}",
                    describe()
                ));
            }
            let requirement = (dependencies_of(name).iter())
                .find(|&&(x, _)| NAMES[x] == dependency.as_ref())
                .map(|&(_, range_index)| range_index);
            if requirement.is_none_or(|x| satisfies(x, dependency_version)) {
                return Err(format!(
                    "{name} is reported as conflicting with {dependency} {dependency_version}, \
                    but it is not\n{}",
                    describe()
                ));
            }
        }
    }

    // We install a prerelease version nobody accepts only to use a version compatible with
    // the unity, so we never do that if all the versions are compatible.
    // Once someone accepts and selects a prerelease version, we keep it even if the requirer is
    // replaced, so we check whether any package in the repository can accept it.
    for (name, change) in result.package_changes() {
        let Some(package) = change.as_install().and_then(|x| x.install_package()) else {
            continue;
        };
        let version = package.version();
        if !version.is_pre() || requests.iter().any(|x| x.name() == name.as_ref()) {
            continue;
        }
        // Prerelease packages accept any prerelease versions.
        let accepted = case.packages.iter().any(|requirer| {
            (requirer.dependencies.iter()).any(|&(dependency, range_index)| {
                NAMES[dependency] == name.as_ref()
                    && (self::version(requirer.version).is_pre()
                        || range(range_index).match_pre(version, PrereleaseAcceptance::Minimum))
            })
        });
        let all_compatible = (case.packages.iter())
            .filter(|x| NAMES[x.name] == name.as_ref())
            .all(|x| !x.new_unity);
        if !accepted && all_compatible {
            return Err(format!(
                "{name} {version} is installed but nobody accepts it\n{}",
                describe()
            ));
        }
    }

    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 4096,
        ..ProptestConfig::default()
    })]

    #[test]
    fn add_package_request(case in case()) {
        run(case)?;
    }
}

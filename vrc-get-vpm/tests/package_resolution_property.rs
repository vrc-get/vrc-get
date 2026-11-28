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

/// The repository and the project.
#[derive(Debug, Clone)]
struct Setup {
    // Sorted from newer to older since the collection for tests returns the first match.
    packages: Vec<PackageSpec>,
    // (name, version, in dependencies)
    project: Vec<(usize, usize, bool)>,
}

#[derive(Debug, Clone)]
struct Case {
    setup: Setup,
    // (name, version)
    requests: Vec<(usize, usize)>,
}

fn setup() -> impl Strategy<Value = Setup> {
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
    )
        .prop_filter_map("no packages", |(versions, project)| {
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

            let mut setup = Setup {
                packages,
                project: vec![],
            };

            // only existing versions can be installed, one version for each package
            let mut project = (project.into_iter())
                .map(|(index, in_dependencies)| {
                    let (name, version) = setup.get(index);
                    (name, version, in_dependencies)
                })
                .collect::<Vec<_>>();
            project.sort_by_key(|x| x.0);
            project.dedup_by_key(|x| x.0);
            setup.project = project;

            Some(setup)
        })
}

fn case() -> impl Strategy<Value = Case> {
    (
        setup(),
        prop::collection::vec(any::<prop::sample::Index>(), 1..=2),
    )
        .prop_map(|(setup, requests)| {
            // only existing versions can be requested, one version for each package
            let mut requests = (requests.into_iter())
                .map(|index| setup.get(index))
                .collect::<Vec<_>>();
            requests.sort_by_key(|x| x.0);
            requests.dedup_by_key(|x| x.0);

            Case { setup, requests }
        })
}

impl Setup {
    /// Returns the name and the version of a package in the repository.
    fn get(&self, index: prop::sample::Index) -> (usize, usize) {
        let package = index.get(&self.packages);
        (package.name, package.version)
    }

    fn spec(&self, name: usize, version: &Version) -> &PackageSpec {
        (self.packages.iter())
            .find(|x| x.name == name && &self::version(x.version) == version)
            .unwrap()
    }

    fn collection(&self) -> PackageCollection {
        let mut collection = PackageCollectionBuilder::new();
        for spec in &self.packages {
            collection = collection.add(manifest(spec));
        }
        collection.build()
    }

    fn project(&self, collection: &PackageCollection) -> VirtualProjectBuilder {
        let mut project = VirtualProjectBuilder::new();
        for &(name, version_index, in_dependencies) in &self.project {
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
        project
    }
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

/// Runs the resolution and checks the result.
///
/// The resolution runs on another thread to fail instead of hanging if it never terminates.
fn run(check: impl FnOnce() -> Result<(), String> + Send + 'static) -> Result<(), TestCaseError> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = check();
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

fn check_add_package_request(case: &Case) -> Result<(), String> {
    let collection = case.setup.collection();
    let project = case.setup.project(&collection);

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
    let outcome = Outcome::new(&case.setup, result);
    let describe = || outcome.describe();

    // The requested versions are installed.
    // If the newer version is installed, we keep it and only add the package to dependencies.
    for request in requests {
        match outcome.installed.get(request.name()) {
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

    outcome.check_requirements()?;
    outcome.check_conflicts()?;

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
        let accepted = case.setup.packages.iter().any(|requirer| {
            (requirer.dependencies.iter()).any(|&(dependency, range_index)| {
                NAMES[dependency] == name.as_ref()
                    && (self::version(requirer.version).is_pre()
                        || range(range_index).match_pre(version, PrereleaseAcceptance::Minimum))
            })
        });
        let all_compatible = (case.setup.packages.iter())
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

fn range(index: usize) -> VersionRange {
    RANGES[index].parse().unwrap()
}

/// Returns whether the version satisfies the range.
///
/// The installed prerelease versions are selected or accepted by someone,
/// so they never conflict with ranges containing them, like the resolution does.
fn satisfies(range_index: usize, version: &Version) -> bool {
    range(range_index).match_pre(version, PrereleaseAcceptance::Allow)
}

/// The project after applying the changes.
struct Outcome<'a> {
    setup: &'a Setup,
    result: &'a PendingProjectChanges<'a>,
    // The packages installed after applying the changes.
    installed: HashMap<&'a str, Version>,
}

impl<'a> Outcome<'a> {
    fn new(setup: &'a Setup, result: &'a PendingProjectChanges<'a>) -> Self {
        let mut installed = HashMap::<&str, Version>::new();
        for &(name, version_index, _) in &setup.project {
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
        Self {
            setup,
            result,
            installed,
        }
    }

    fn describe(&self) -> impl std::fmt::Display {
        describe_changes(self.result)
    }

    /// Returns whether the package is installed or removed.
    fn changed(&self, name: &str) -> bool {
        (self.result.package_changes().get(name)).is_some_and(|change| {
            change.as_remove().is_some()
                || (change.as_install()).is_some_and(|x| x.install_package().is_some())
        })
    }

    fn dependencies_of(&self, name: &str) -> &'a [(usize, usize)] {
        let index = NAMES.iter().position(|&x| x == name).unwrap();
        &self.setup.spec(index, &self.installed[name]).dependencies
    }

    /// Checks the requirements related to the changes are satisfied, or reported as conflicts.
    fn check_requirements(&self) -> Result<(), String> {
        for &name in self.installed.keys() {
            for &(dependency, range_index) in self.dependencies_of(name) {
                let dependency = NAMES[dependency];
                if !self.changed(name) && !self.changed(dependency) {
                    // We never check the requirements we don't touch.
                    continue;
                }
                let Some(dependency_version) = self.installed.get(dependency) else {
                    return Err(format!(
                        "{dependency} required by {name} is not installed\n{}",
                        self.describe()
                    ));
                };
                let reported = (self.result.conflicts().get(dependency))
                    .is_some_and(|x| (x.conflicting_packages().iter()).any(|x| x.as_ref() == name));
                if !satisfies(range_index, dependency_version) && !reported {
                    return Err(format!(
                        "{dependency} {dependency_version} doesn't satisfy {} required by {name}, \
                        but not reported as a conflict\n{}",
                        RANGES[range_index],
                        self.describe()
                    ));
                }
            }
        }
        Ok(())
    }

    /// Checks the reported conflicts are real.
    fn check_conflicts(&self) -> Result<(), String> {
        for (dependency, conflict) in self.result.conflicts() {
            // We report the existing conflicts only with new conflicts of the same dependency,
            // and never re-report the existing conflicts the changes are unrelated to.
            let has_new_conflict = self.changed(dependency)
                || (conflict.conflicting_packages().iter()).any(|name| self.changed(name));
            if !has_new_conflict {
                return Err(format!(
                    "{dependency} is reported as conflict but has no new conflict\n{}",
                    self.describe()
                ));
            }
            for name in conflict.conflicting_packages() {
                if name.is_empty() {
                    // The requirement from the project
                    continue;
                }
                let Some(dependency_version) = self.installed.get(dependency.as_ref()) else {
                    return Err(format!(
                        "{dependency} is reported as conflict but not installed\n{}",
                        self.describe()
                    ));
                };
                if !self.installed.contains_key(name.as_ref()) {
                    return Err(format!(
                        "{name} is reported as conflicting with {dependency} but not installed\n{}",
                        self.describe()
                    ));
                }
                let requirement = (self.dependencies_of(name).iter())
                    .find(|&&(x, _)| NAMES[x] == dependency.as_ref())
                    .map(|&(_, range_index)| range_index);
                if requirement.is_none_or(|x| satisfies(x, dependency_version)) {
                    return Err(format!(
                        "{name} is reported as conflicting with {dependency} {dependency_version}, \
                        but it is not\n{}",
                        self.describe()
                    ));
                }
            }
        }
        Ok(())
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 4096,
        ..ProptestConfig::default()
    })]

    #[test]
    fn add_package_request(case in case()) {
        run(move || check_add_package_request(&case))?;
    }
}

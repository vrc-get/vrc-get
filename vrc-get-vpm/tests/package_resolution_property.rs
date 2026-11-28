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
//! We generate small random repositories, projects, and requests,
//! and check the resolution terminates and the result is consistent.
//!
//! Only which versions a range accepts matters, so we use a few versions and ranges.

use common::*;
use proptest::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::time::Duration;
use vrc_get_vpm::PackageManifest;
use vrc_get_vpm::unity_project::pending_project_changes::RemoveReason;
use vrc_get_vpm::unity_project::{AddPackageErr, RemovePackageErr, ResolvePackageErr};
use vrc_get_vpm::unity_project::{AddPackageOperation, PendingProjectChanges};
use vrc_get_vpm::version::{PrereleaseAcceptance, Version, VersionRange};

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

/// How a package is in the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Locked {
        in_dependencies: bool,
    },
    /// Installed but neither in locked nor in dependencies.
    Unlocked,
    /// Only in dependencies like template projects.
    DependencyOnly,
    /// In dependencies but not in locked, and installed as an unlocked package.
    ///
    /// The project is in an invalid state. Currently, resolve replaces the unlocked package
    /// with the version resolved for the dependency, but it's not a behavior we guarantee.
    UnlockedDependencyOnly,
}

impl State {
    fn is_installed(self) -> bool {
        self != State::DependencyOnly
    }

    fn in_dependencies(self) -> bool {
        match self {
            State::Locked { in_dependencies } => in_dependencies,
            State::Unlocked => false,
            State::DependencyOnly | State::UnlockedDependencyOnly => true,
        }
    }
}

const ALL_STATES: &[State] = &[
    State::Locked {
        in_dependencies: false,
    },
    State::Locked {
        in_dependencies: true,
    },
    State::Unlocked,
    State::DependencyOnly,
    State::UnlockedDependencyOnly,
];

/// The repository and the project.
#[derive(Debug, Clone)]
struct Setup {
    // Sorted from newer to older since the collection for tests returns the first match.
    packages: Vec<PackageSpec>,
    // (name, version, state)
    project: Vec<(usize, usize, State)>,
}

#[derive(Debug, Clone)]
struct Case {
    setup: Setup,
    // (name, version)
    requests: Vec<(usize, usize)>,
}

fn setup(states: &'static [State]) -> impl Strategy<Value = Setup> {
    // Most versions exist and depend on most other packages, so versions are often replaced,
    // which is where the resolution gets complicated.
    let version = (
        prop::bool::weighted(0.85),
        prop::collection::vec(prop::option::weighted(0.7, 0..RANGES.len()), NAMES.len()),
        prop::bool::weighted(0.2),
    );
    (
        prop::collection::vec(version, NAMES.len() * VERSIONS.len()),
        // Each package is in the project in most cases, with one of the versions and states.
        prop::collection::vec(
            prop::option::weighted(
                0.7,
                (any::<prop::sample::Index>(), prop::sample::select(states)),
            ),
            NAMES.len(),
        ),
    )
        .prop_filter_map("no packages", move |(versions, project)| {
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

            // only existing versions can be in the project
            setup.project = (project.into_iter().enumerate())
                .filter_map(|(name, entry)| {
                    let (index, state) = entry?;
                    let versions = (setup.packages.iter())
                        .filter(|x| x.name == name)
                        .collect::<Vec<_>>();
                    if versions.is_empty() {
                        return None;
                    }
                    Some((name, index.get(&versions).version, state))
                })
                .collect::<Vec<_>>();

            Some(setup)
        })
}

fn case() -> impl Strategy<Value = Case> {
    (
        setup(ALL_STATES),
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

#[derive(Debug, Clone)]
struct RemoveCase {
    setup: Setup,
    // names, which can be not locked or not in the project
    requests: Vec<usize>,
}

fn remove_case() -> impl Strategy<Value = RemoveCase> {
    (
        setup(ALL_STATES),
        prop::collection::vec(0..NAMES.len(), 1..=2),
    )
        .prop_map(|(setup, mut requests)| {
            requests.sort();
            requests.dedup();
            RemoveCase { setup, requests }
        })
}

impl Setup {
    fn state_of(&self, name: usize) -> Option<State> {
        (self.project.iter()).find(|x| x.0 == name).map(|x| x.2)
    }

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
        for &(name, version_index, state) in &self.project {
            let package = collection.get_package(NAMES[name], version(version_index));
            match state {
                State::Locked { .. } => {
                    let dependencies = (package.vpm_dependencies().iter())
                        .map(|(name, range)| (name.as_ref(), range.to_string()))
                        .collect::<Vec<_>>();
                    let dependencies = (dependencies.iter())
                        .map(|(name, range)| (*name, range.as_str()))
                        .collect::<Vec<_>>();
                    project.add_locked(NAMES[name], version(version_index), &dependencies);
                }
                State::Unlocked | State::UnlockedDependencyOnly => {
                    let spec = self.spec(name, &version(version_index));
                    project.add_package_json(NAMES[name], manifest_json(spec).to_string());
                }
                State::DependencyOnly => {}
            }
            if state.in_dependencies() {
                project.add_dependency(NAMES[name], version(version_index));
            }
        }
        project
    }
}

fn manifest_json(spec: &PackageSpec) -> serde_json::Value {
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
    json
}

fn manifest(spec: &PackageSpec) -> PackageManifest {
    serde_json::from_value(manifest_json(spec)).unwrap()
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

    let result = block_on(async {
        let project = (project.build_named("add_package_request").await).unwrap();
        project
            .add_package_request(
                &collection,
                &requests,
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
    });
    let result = match result {
        Ok(result) => result,
        // No version of some dependency exists.
        Err(AddPackageErr::DependenciesNotFound { .. }) => return Ok(()),
        Err(e) => return Err(format!("unexpected error: {e}")),
    };

    let requested = (case.requests.iter())
        .map(|&(name, _)| NAMES[name])
        .collect();
    let outcome = Outcome::new(&case.setup, requested, &result);
    let describe = || outcome.describe();
    outcome.check_common()?;
    outcome.check_unused_removed()?;

    // The requested versions are installed.
    // If the newer version is installed, we keep it and only add the package to dependencies.
    // The dependencies of the packages we downgrade can downgrade it further,
    // which is reported as a conflict with the project.
    for request in &requests {
        let installed = outcome.installed.get(request.name());
        let conflicts_with_project = (result.conflicts().get(request.name()))
            .is_some_and(|x| (x.conflicting_packages().iter()).any(|x| x.is_empty()));
        if !installed.is_some_and(|x| x >= request.version() || conflicts_with_project) {
            return Err(format!(
                "requested {} {} is installed with {installed:?}\n{}",
                request.name(),
                request.version(),
                describe()
            ));
        }
    }

    // The unlocked packages we replace are reported.
    for &name in outcome.before.keys() {
        if !outcome.is_unlocked(name) || !outcome.changed(name) {
            continue;
        }
        let reported = (result.conflicts().get(name))
            .is_some_and(|x| x.unlocked_names().iter().any(|x| x.as_ref() == name));
        if !reported {
            return Err(format!(
                "unlocked {name} is replaced but not reported\n{}",
                describe()
            ));
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
        if !version.is_pre() || outcome.is_requested(name) {
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

fn check_resolve_request(setup: &Setup) -> Result<(), String> {
    let collection = setup.collection();
    let project = setup.project(&collection);

    let result = block_on(async {
        let project = (project.build_named("resolve_request").await).unwrap();
        project.resolve_request(&collection).await
    });
    let result = match result {
        Ok(result) => result,
        // No version of some dependency exists.
        Err(ResolvePackageErr::DependenciesNotFound { .. }) => return Ok(()),
        Err(e) => return Err(format!("unexpected error: {e}")),
    };

    // We install the dependency-only packages like the packages the user requests to add.
    let requested = (setup.project.iter())
        .filter(|x| matches!(x.2, State::DependencyOnly | State::UnlockedDependencyOnly))
        .map(|x| NAMES[x.0])
        .collect();
    let outcome = Outcome::new(setup, requested, &result);
    let describe = || outcome.describe();
    outcome.check_common()?;

    // The locked packages are installed.
    for &name in outcome.before.keys() {
        if !outcome.is_locked(name) {
            continue;
        }
        let installing = (result.package_changes().get(name))
            .and_then(|x| x.as_install())
            .is_some_and(|x| x.install_package().is_some());
        if !installing {
            return Err(format!("locked {name} is not installed\n{}", describe()));
        }
    }

    // We change the versions of the installed packages only to install the missing packages:
    // the dependency-only packages, and the dependencies of the unlocked packages not installed.
    let missing = !outcome.requested.is_empty()
        || (outcome.before.keys()).any(|&name| {
            outcome.state_of(name) == Some(State::Unlocked)
                && (outcome.dependencies_of(name).iter())
                    .any(|&(dependency, _)| !outcome.before.contains_key(NAMES[dependency]))
        });
    if !missing {
        for (&name, before) in &outcome.before {
            if outcome.installed.get(name) != Some(before) {
                return Err(format!(
                    "{name} {before} is changed but nothing is missing\n{}",
                    describe()
                ));
            }
        }
    }

    // The dependency-only packages are installed.
    for &name in &outcome.requested {
        if !outcome.installed.contains_key(name) {
            return Err(format!(
                "dependency-only {name} is not installed\n{}",
                describe()
            ));
        }
    }

    // The dependencies of the unlocked packages are installed.
    for &name in outcome.before.keys() {
        if outcome.state_of(name) != Some(State::Unlocked) {
            continue;
        }
        for &(dependency, _) in outcome.dependencies_of(name) {
            let dependency = NAMES[dependency];
            if !outcome.installed.contains_key(dependency) {
                return Err(format!(
                    "{dependency} required by unlocked {name} is not installed\n{}",
                    describe()
                ));
            }
        }
    }

    // Currently, we replace the dependency-only packages installed as unlocked packages.
    for &name in outcome.before.keys() {
        if outcome.state_of(name) != Some(State::UnlockedDependencyOnly) {
            continue;
        }
        let replaced = (result.package_changes().get(name))
            .and_then(|x| x.as_install())
            .is_some_and(|x| x.install_package().is_some() && x.is_adding_to_locked());
        if !replaced {
            return Err(format!(
                "unlocked dependency-only {name} is not replaced\n{}",
                describe()
            ));
        }
    }

    Ok(())
}

fn check_remove_request(case: &RemoveCase) -> Result<(), String> {
    let collection = case.setup.collection();
    let project = case.setup.project(&collection);
    let requests = (case.requests.iter())
        .map(|&name| NAMES[name])
        .collect::<Vec<_>>();
    // Only locked packages can be removed.
    let not_locked = (case.requests.iter())
        .filter(|&&name| !matches!(case.setup.state_of(name), Some(State::Locked { .. })))
        .map(|&name| NAMES[name])
        .collect::<HashSet<_>>();

    let result = block_on(async {
        let project = (project.build_named("remove_request").await).unwrap();
        project.remove_request(&requests).await
    });
    let result = match result {
        Ok(result) if not_locked.is_empty() => result,
        Ok(result) => {
            return Err(format!(
                "{not_locked:?} are not locked but removed\n{}",
                describe_changes(&result)
            ));
        }
        Err(RemovePackageErr::NotInstalled(names)) => {
            let names = names.iter().map(|x| x.as_ref()).collect::<HashSet<_>>();
            if names == not_locked {
                return Ok(());
            }
            return Err(format!(
                "{names:?} are reported as not installed, but {not_locked:?} are not locked"
            ));
        }
        Err(e) => return Err(format!("unexpected error: {e}")),
    };

    let outcome = Outcome::new(&case.setup, requests, &result);
    let describe = || outcome.describe();
    outcome.check_common()?;
    outcome.check_unused_removed()?;

    // The requested packages are removed.
    for &name in &outcome.requested {
        if outcome.installed.contains_key(name) {
            return Err(format!("requested {name} is not removed\n{}", describe()));
        }
    }

    // We only remove packages, the requested ones as requested and others as unused.
    for (name, change) in result.package_changes() {
        let expected = if outcome.is_requested(name) {
            RemoveReason::Requested
        } else {
            RemoveReason::Unused
        };
        if change.as_remove().is_none_or(|x| x.reason() != expected) {
            return Err(format!(
                "{name} is not removed as {expected:?}\n{}",
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

/// The project before and after applying the changes.
struct Outcome<'a> {
    setup: &'a Setup,
    result: &'a PendingProjectChanges<'a>,
    // The packages the user requested to add or remove.
    requested: Vec<&'static str>,
    // The packages installed before applying the changes, locked or unlocked.
    before: HashMap<&'a str, Version>,
    // The packages installed after applying the changes.
    installed: HashMap<&'a str, Version>,
}

impl<'a> Outcome<'a> {
    fn new(
        setup: &'a Setup,
        requested: Vec<&'static str>,
        result: &'a PendingProjectChanges<'a>,
    ) -> Self {
        let mut before = HashMap::<&str, Version>::new();
        for &(name, version_index, state) in &setup.project {
            if state.is_installed() {
                before.insert(NAMES[name], version(version_index));
            }
        }
        let mut installed = before.clone();
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
            requested,
            before,
            installed,
        }
    }

    fn describe(&self) -> impl std::fmt::Display {
        describe_changes(self.result)
    }

    fn state_of(&self, name: &str) -> Option<State> {
        (self.setup.project.iter())
            .find(|x| NAMES[x.0] == name)
            .map(|x| x.2)
    }

    fn is_requested(&self, name: &str) -> bool {
        self.requested.contains(&name)
    }

    fn is_locked(&self, name: &str) -> bool {
        matches!(self.state_of(name), Some(State::Locked { .. }))
    }

    fn is_unlocked(&self, name: &str) -> bool {
        matches!(
            self.state_of(name),
            Some(State::Unlocked | State::UnlockedDependencyOnly)
        )
    }

    /// Returns whether the package is in the dependencies of the project, before or after
    /// applying the changes.
    fn in_dependencies(&self, name: &str) -> bool {
        let adding = (self.result.package_changes().get(name))
            .and_then(|x| x.as_install())
            .is_some_and(|x| x.to_dependencies().is_some());
        self.state_of(name).is_some_and(State::in_dependencies) || adding
    }

    /// Returns whether the package is installed, removed, replaced with another version,
    /// or added to locked.
    fn changed(&self, name: &str) -> bool {
        self.before.get(name) != self.installed.get(name)
            || (self.result.package_changes().get(name))
                .and_then(|x| x.as_install())
                .is_some_and(|x| x.is_adding_to_locked())
    }

    /// Returns the dependencies of the package installed after applying the changes.
    fn dependencies_of(&self, name: &str) -> &'a [(usize, usize)] {
        let index = NAMES.iter().position(|&x| x == name).unwrap();
        &self.setup.spec(index, &self.installed[name]).dependencies
    }

    /// Returns the dependencies of the package installed before applying the changes.
    fn dependencies_before(&self, name: &str) -> &'a [(usize, usize)] {
        let index = NAMES.iter().position(|&x| x == name).unwrap();
        &self.setup.spec(index, &self.before[name]).dependencies
    }

    /// Returns the packages requiring the package after applying the changes.
    fn requirers_of(&self, name: &str) -> Vec<&'a str> {
        (self.installed.keys())
            .filter(|&&x| (self.dependencies_of(x).iter()).any(|&(y, _)| NAMES[y] == name))
            .copied()
            .collect()
    }

    /// Checks the properties every operation keeps.
    fn check_common(&self) -> Result<(), String> {
        // The requirements related to the changes are satisfied, or reported as conflicts.
        for &name in self.installed.keys() {
            for &(dependency, range_index) in self.dependencies_of(name) {
                let dependency = NAMES[dependency];
                if !self.changed(name) && !self.changed(dependency) {
                    // We never check the requirements we don't touch.
                    continue;
                }
                let reported = (self.result.conflicts().get(dependency))
                    .is_some_and(|x| (x.conflicting_packages().iter()).any(|x| x.as_ref() == name));
                if reported {
                    continue;
                }
                let Some(dependency_version) = self.installed.get(dependency) else {
                    return Err(format!(
                        "{dependency} required by {name} is not installed, \
                        but not reported as a conflict\n{}",
                        self.describe()
                    ));
                };
                if !satisfies(range_index, dependency_version) {
                    return Err(format!(
                        "{dependency} {dependency_version} doesn't satisfy {} required by {name}, \
                        but not reported as a conflict\n{}",
                        RANGES[range_index],
                        self.describe()
                    ));
                }
            }
        }

        // The reported conflicts are real.
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
                if !self.installed.contains_key(name.as_ref()) {
                    return Err(format!(
                        "{name} is reported as conflicting with {dependency} but not installed\n{}",
                        self.describe()
                    ));
                }
                let Some(&(_, range_index)) = (self.dependencies_of(name).iter())
                    .find(|&&(x, _)| NAMES[x] == dependency.as_ref())
                else {
                    return Err(format!(
                        "{name} is reported as conflicting with {dependency}, \
                        but doesn't require it\n{}",
                        self.describe()
                    ));
                };
                // The dependency is satisfied if it's installed with a version in the range.
                let satisfied = (self.installed.get(dependency.as_ref()))
                    .is_some_and(|version| satisfies(range_index, version));
                if satisfied {
                    return Err(format!(
                        "{name} is reported as conflicting with {dependency}, but it is not\n{}",
                        self.describe()
                    ));
                }
            }
        }

        // The unlocked packages are neither replaced nor removed unless the user requests.
        for &name in self.before.keys() {
            if self.is_unlocked(name) && !self.is_requested(name) && self.changed(name) {
                return Err(format!("unlocked {name} is changed\n{}", self.describe()));
            }
        }

        // The packages we newly install are requested, in the dependencies of the project,
        // or required by installed packages.
        for &name in self.installed.keys() {
            if self.before.contains_key(name) {
                continue;
            }
            let needed = self.is_requested(name)
                || self.in_dependencies(name)
                || !self.requirers_of(name).is_empty();
            if !needed {
                return Err(format!(
                    "{name} is installed but nobody needs it\n{}",
                    self.describe()
                ));
            }
        }

        // The packages we remove are requested, or neither in the dependencies of the project
        // nor required by the packages we keep.
        for &name in self.before.keys() {
            if self.installed.contains_key(name) || self.is_requested(name) {
                continue;
            }
            if self.in_dependencies(name) {
                return Err(format!(
                    "{name} is removed but in the dependencies\n{}",
                    self.describe()
                ));
            }
            if let Some(requirer) = self.requirers_of(name).first() {
                return Err(format!(
                    "{name} is removed but required by {requirer}\n{}",
                    self.describe()
                ));
            }
        }

        // We downgrade packages only along with the requested packages we downgrade.
        // The downgrades may remain after we replace the packages requiring them,
        // so we only check some requested package is downgraded.
        let requested_downgraded = (self.requested.iter()).any(|&name| {
            (self.before.get(name)).is_some_and(|before| {
                (self.installed.get(name)).is_some_and(|after| after < before)
            })
        });
        for (&name, before) in &self.before {
            let Some(after) = self.installed.get(name) else {
                continue;
            };
            if after < before && !self.is_requested(name) && !requested_downgraded {
                return Err(format!(
                    "{name} {before} is downgraded to {after}, \
                    but no requested package is downgraded\n{}",
                    self.describe()
                ));
            }
        }

        Ok(())
    }

    /// Checks the packages nobody uses anymore are removed.
    ///
    /// Only the packages the removed or replaced packages required can become unused,
    /// and we keep the packages unused before.
    fn check_unused_removed(&self) -> Result<(), String> {
        // The packages which may become unused: the removed or replaced packages,
        // and the packages they required before, recursively.
        // We don't know what the unlocked packages required, since it's not recorded in locked.
        let mut removable = HashSet::new();
        let mut queue = (self.before.keys())
            .filter(|&&x| self.installed.get(x) != self.before.get(x))
            .copied()
            .collect::<Vec<_>>();
        while let Some(name) = queue.pop() {
            if !removable.insert(name) || !self.is_locked(name) {
                continue;
            }
            queue.extend(self.dependencies_before(name).iter().map(|x| NAMES[x.0]));
        }

        // The packages in use: the packages in the dependencies of the project,
        // the unlocked packages, the packages which can't become unused,
        // and the packages they require now, recursively.
        let mut in_use = HashSet::new();
        let mut queue = (self.installed.keys())
            .filter(|&&x| self.in_dependencies(x) || self.is_unlocked(x) || !removable.contains(x))
            .copied()
            .collect::<Vec<_>>();
        while let Some(name) = queue.pop() {
            if !in_use.insert(name) || !self.installed.contains_key(name) {
                continue;
            }
            queue.extend(self.dependencies_of(name).iter().map(|x| NAMES[x.0]));
        }

        // The locked packages which became unused are removed.
        for &name in self.before.keys() {
            if !self.is_locked(name) || self.is_requested(name) {
                continue;
            }
            let unused = removable.contains(name) && !in_use.contains(name);
            if unused && self.installed.contains_key(name) {
                return Err(format!(
                    "{name} is unused but not removed\n{}",
                    self.describe()
                ));
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

    #[test]
    fn resolve_request(setup in setup(ALL_STATES)) {
        run(move || check_resolve_request(&setup))?;
    }

    #[test]
    fn remove_request(case in remove_case()) {
        run(move || check_remove_request(&case))?;
    }
}

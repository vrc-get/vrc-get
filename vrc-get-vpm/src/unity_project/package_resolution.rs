/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

use crate::traits::PackageCollection;
use crate::unity_project::LockedDependencyInfo;
use crate::version::{DependencyRange, PrereleaseAcceptance, UnityVersion, Version, VersionRange};
use crate::{PackageInfo, PackageManifest, VersionSelector};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet, VecDeque};

/// The package requested to install from outside of the dependency resolution.
pub(crate) enum Request<'env, 'a> {
    /// The package the user selected the version of.
    ///
    /// We always install this version, and never replace it with another version.
    Package(PackageInfo<'env>),
    /// The locked package the user selected the locked version of.
    ///
    /// We keep the locked version, and never replace it with another version.
    KeepLocked(&'a str),
    /// The package required by the project or installed packages, but not installed.
    ///
    /// We select the version in the same way as dependencies of installing packages,
    /// so we can replace it with another version to satisfy other packages.
    Range {
        name: &'a str,
        range: VersionRange,
        // The acceptances of prerelease versions we try in order.
        // We select the newest version with the first acceptance some version satisfies.
        prerelease: &'a [PrereleaseAcceptance],
    },
}

/// The one that requires a package; an installing package or the one that made a request.
struct Requirer<'a> {
    // The name of the package. This is empty for requests.
    name: &'a str,
    // The acceptances of prerelease versions we try in order to select the dependencies.
    prerelease: &'a [PrereleaseAcceptance],
    // Dependencies are downgraded only along with the package that requires them.
    // Installing or upgrading a package never downgrades other packages;
    // the unsatisfied requirement is reported as a conflict instead.
    allow_downgrade: bool,
}

struct PackageQueue<'a> {
    force_count: usize,
    pending_queue: VecDeque<PackageInfo<'a>>,
}

impl<'a> PackageQueue<'a> {
    fn new(packages: Vec<PackageInfo<'a>>) -> Self {
        Self {
            force_count: packages.len(),
            pending_queue: VecDeque::from_iter(packages),
        }
    }

    pub(crate) fn next_package(&mut self) -> Option<(PackageInfo<'a>, bool)> {
        let force = self.force_count > 0;
        self.force_count = self.force_count.saturating_sub(1);
        self.pending_queue.pop_front().map(|x| (x, force))
    }

    fn find_pending_package(&self, name: &str) -> Option<&PackageInfo<'a>> {
        self.pending_queue.iter().find(|x| x.name() == name)
    }

    pub(crate) fn add_pending_package(&mut self, package: PackageInfo<'a>) {
        self.pending_queue.retain(|x| x.name() != package.name());
        self.pending_queue.push_back(package);
    }
}

struct ResolutionContext<'env, 'a>
where
    'env: 'a,
{
    allow_prerelease: bool,
    pub pending_queue: PackageQueue<'env>,
    dependencies: HashMap<&'a str, DependencyInfo<'env, 'a>>,
    unlocked_names: HashSet<&'a str>,
    // The packages the user selected the version of.
    // We never replace them with another version.
    requested: HashSet<&'a str>,
    // The installed packages that become unused and should be removed.
    unused: Vec<&'a str>,
    // The dependencies we couldn't find, with the requirer ("" for requests).
    // The requirer can be replaced or stop being used later, so we keep the requirer
    // to report only the dependencies of the packages we actually install.
    missing: Vec<(&'a str, &'a str, &'a VersionRange)>,
}

struct Legacy<'env>(&'env [Box<str>]);

impl Default for Legacy<'_> {
    fn default() -> Self {
        static VEC: Vec<Box<str>> = Vec::new();
        Self(&VEC)
    }
}

#[derive(Default)]
struct DependencyInfo<'env, 'a> {
    using: Option<PackageInfo<'env>>,
    current: Option<&'a Version>,
    // The version installed before this resolution. Unlike `current`, this never changes.
    installed: Option<&'a Version>,
    // "" key for root dependencies
    requirements: HashMap<&'a str, &'a VersionRange>,
    // The sources of requirements that were not satisfied by the installed version
    // before this resolution, in other words, the requirements the user is already ignoring.
    ignored_sources: HashSet<&'a str>,
    dependencies: HashSet<&'a str>,

    modern_packages: HashSet<&'a str>,
    legacy_packages: Legacy<'a>,

    allow_pre: bool,
    touched: bool,
}

impl<'env, 'a> DependencyInfo<'env, 'a>
where
    'env: 'a,
{
    fn new_dependency(version_range: &'a VersionRange, allow_pre: bool) -> Self {
        let mut requirements = HashMap::new();
        requirements.insert("", version_range);
        DependencyInfo {
            using: None,
            current: None,
            installed: None,
            requirements,
            ignored_sources: HashSet::new(),
            dependencies: HashSet::new(),

            modern_packages: HashSet::new(),
            legacy_packages: Legacy::default(),

            allow_pre,
            touched: false,
        }
    }

    fn add_range(&mut self, source: &'a str, range: &'a VersionRange) {
        self.requirements.insert(source, range);
        self.touched = true;
    }

    fn remove_range(&mut self, source: &str) {
        self.requirements.remove(source);
        self.touched = true;
    }

    pub(crate) fn add_modern_package(&mut self, modern: &'a str) {
        self.modern_packages.insert(modern);
        self.touched = true;
    }

    pub(crate) fn remove_modern_package(&mut self, modern: &'a str) {
        self.modern_packages.remove(modern);
        self.touched = true;
    }

    pub fn is_legacy(&self) -> bool {
        !self.modern_packages.is_empty()
    }

    pub(crate) fn set_using_info(&mut self, version: &'a Version, dependencies: HashSet<&'a str>) {
        self.allow_pre |= !version.pre.is_empty();
        self.current = Some(version);
        self.installed = Some(version);
        self.dependencies = dependencies;
    }
}

impl<'env> ResolutionContext<'env, '_> {
    fn new(allow_prerelease: bool, packages: Vec<PackageInfo<'env>>) -> Self {
        let requested = packages.iter().map(|pkg| pkg.name()).collect();
        let mut this = Self {
            dependencies: HashMap::new(),
            pending_queue: PackageQueue::new(packages),
            allow_prerelease,
            unlocked_names: HashSet::new(),
            requested,
            unused: Vec::new(),
            missing: Vec::new(),
        };

        for pkg in &this.pending_queue.pending_queue {
            this.dependencies.entry(pkg.name()).or_default().allow_pre = true;
        }
        this.dependencies.insert("", DependencyInfo::default());

        this
    }
}

impl<'env, 'a> ResolutionContext<'env, 'a>
where
    'env: 'a,
{
    pub(crate) fn add_root_dependency(
        &mut self,
        name: &'a str,
        range: &'a VersionRange,
        allow_pre: bool,
    ) {
        self.dependencies
            .get_mut("")
            .unwrap()
            .requirements
            .insert(name, range);
        self.dependencies
            .insert(name, DependencyInfo::new_dependency(range, allow_pre));
    }

    pub(crate) fn add_unlocked_name(&mut self, name: &'a str) {
        self.unlocked_names.insert(name);
    }

    pub(crate) fn add_unlocked_manifest(&mut self, manifest: &'a PackageManifest) {
        let info = self.dependencies.entry(manifest.name()).or_default();
        info.set_using_info(
            manifest.version(),
            manifest
                .vpm_dependencies()
                .keys()
                .map(|x| x.as_ref())
                .collect(),
        );

        info.legacy_packages = Legacy(manifest.legacy_packages());

        for legacy in manifest.legacy_packages() {
            self.dependencies
                .entry(legacy)
                .or_default()
                .modern_packages
                .insert(manifest.name());
        }

        for (dependency, range) in manifest.vpm_dependencies() {
            self.dependencies
                .entry(dependency)
                .or_default()
                .requirements
                .insert(manifest.name(), range);
        }
    }

    pub(crate) fn add_locked_dependency(
        &mut self,
        locked: LockedDependencyInfo<'a>,
        env: &'env impl PackageCollection,
    ) {
        let info = self.dependencies.entry(locked.name()).or_default();
        info.set_using_info(
            locked.version(),
            locked.dependencies().keys().map(|x| x.as_ref()).collect(),
        );

        if let Some(pkg) = env.find_package_by_name(
            locked.name(),
            VersionSelector::specific_version(locked.version()),
        ) {
            info.legacy_packages = Legacy(pkg.legacy_packages());

            for legacy in pkg.legacy_packages() {
                self.dependencies
                    .entry(legacy)
                    .or_default()
                    .modern_packages
                    .insert(locked.name());
            }
        }

        for (dependency, range) in locked.dependencies() {
            self.dependencies
                .entry(dependency)
                .or_default()
                .requirements
                .insert(locked.name(), range);
        }
    }

    pub(crate) fn add_package(&mut self, package: PackageInfo<'env>, force: bool) -> bool {
        let entry = self.dependencies.entry(package.name()).or_default();

        if entry.is_legacy() {
            return false;
        }

        if !force && self.unlocked_names.contains(package.name()) {
            return false;
        }

        entry.touched = true;
        let vpm_dependencies = &package.vpm_dependencies();
        let legacy_packages = package.legacy_packages();
        let name = package.name();

        // We selected the prerelease version for someone, so it must not be a conflict.
        entry.allow_pre |= package.version().is_pre();
        entry.current = Some(package.version());
        entry.using = Some(package);

        let old_dependencies = std::mem::replace(
            &mut entry.dependencies,
            vpm_dependencies.keys().map(|x| x.as_ref()).collect(),
        );
        let old_legacy_packages =
            std::mem::replace(&mut entry.legacy_packages, Legacy(legacy_packages));

        // region process dependencies
        // remove previous dependencies if exists
        for dep in &old_dependencies {
            self.dependencies.get_mut(*dep).unwrap().remove_range(name);
        }
        self.missing.retain(|&(requirer, _, _)| requirer != name);
        for (dependency, range) in vpm_dependencies.iter() {
            self.dependencies
                .entry(dependency)
                .or_default()
                .add_range(name, range)
        }
        // endregion

        // region process modern packages
        for dep in old_legacy_packages.0 {
            self.dependencies
                .get_mut(dep.as_ref())
                .unwrap()
                .remove_modern_package(name);
        }
        for legacy in legacy_packages {
            self.dependencies
                .entry(legacy)
                .or_default()
                .add_modern_package(name)
        }
        // endregion

        true
    }

    pub(crate) fn should_add_package(&self, name: &'a str, range: &'a VersionRange) -> bool {
        let entry = self.dependencies.get(name).unwrap();

        if entry.is_legacy() {
            log::debug!(
                "processing package {name}: dependency {name} version {range}: legacy package"
            );
            return false;
        }

        if self.unlocked_names.contains(name) {
            log::debug!(
                "processing package {name}: dependency {name} version {range}: unlocked package"
            );
            return false;
        }

        // The user requested the version so we must not replace it with another version.
        // The requirement is reported as a conflict if the requested version doesn't satisfy it.
        if self.requested.contains(name) {
            log::debug!(
                "processing package {name}: dependency {name} version {range}: requested package"
            );
            return false;
        }

        let mut install = true;
        let allow_prerelease =
            PrereleaseAcceptance::allow_or_minimum(entry.allow_pre || self.allow_prerelease);

        if let Some(pending) = self.pending_queue.find_pending_package(name) {
            if range.match_pre(pending.version(), allow_prerelease) {
                // if installing version is good, no need to reinstall
                install = false;
                log::debug!(
                    "processing package {name}: dependency {name} version {range}: pending matches"
                );
            }
        } else {
            // if already installed version is good, no need to reinstall
            if let Some(version) = &entry.current
                && range.match_pre(version, allow_prerelease)
            {
                log::debug!(
                    "processing package {name}: dependency {name} version {range}: existing matches"
                );
                install = false;
            }
        }

        install
    }

    /// Returns true if the requirements from the `source` should be satisfied.
    fn is_active_source(&self, source: &str) -> bool {
        self.dependencies
            .get(source)
            .map(|x| !x.is_legacy())
            .unwrap_or_default()
    }

    /// Remembers the requirements that are not satisfied by the installed packages.
    ///
    /// This must be called after adding all installed packages and before adding new packages.
    pub(crate) fn mark_ignored_requirements(&mut self) {
        for info in self.dependencies.values_mut() {
            let Some(installed) = info.installed else {
                continue;
            };
            let allow_prerelease =
                PrereleaseAcceptance::allow_or_minimum(info.allow_pre || self.allow_prerelease);

            info.ignored_sources = info
                .requirements
                .iter()
                .filter(|(_, range)| !range.match_pre(installed, allow_prerelease))
                .map(|(&source, _)| source)
                .collect();
        }
    }

    /// Returns all requirements for the package we currently know.
    fn requirements_for(&self, name: &str) -> Vec<&'a VersionRange> {
        self.dependencies[name]
            .requirements
            .iter()
            .filter(|&(&source, _)| self.is_active_source(source))
            .map(|(_, &range)| range)
            .collect()
    }

    /// Returns the requirements for the package except for ones the user is already ignoring.
    ///
    /// The requirement is ignored if it was not satisfied before this resolution and
    /// the installed version still doesn't satisfy it, like exceeded upper bound of
    /// supported versions. The requirement from the `requester` is never ignored.
    fn unignored_requirements_for(&self, name: &str, requester: &str) -> Vec<&'a VersionRange> {
        let info = &self.dependencies[name];
        let allow_prerelease =
            PrereleaseAcceptance::allow_or_minimum(info.allow_pre || self.allow_prerelease);

        info.requirements
            .iter()
            .filter(|&(&source, _)| self.is_active_source(source))
            .filter(|&(&source, &range)| {
                source == requester
                    || !info.ignored_sources.contains(source)
                    || info
                        .installed
                        .is_none_or(|installed| range.match_pre(installed, allow_prerelease))
            })
            .map(|(_, &range)| range)
            .collect()
    }

    /// Returns true if installing the `version` lowers the version of the installed package.
    pub(crate) fn is_downgrade(&self, name: &str, version: &Version) -> bool {
        self.dependencies
            .get(name)
            .and_then(|info| info.installed)
            .is_some_and(|installed| version < installed)
    }

    /// Stops using the packages that became unnecessary while resolving.
    ///
    /// A package can be replaced with another version after we process its dependencies.
    /// The packages only the replaced version requires are still marked as using then.
    /// A package can also get back to the installed version, which needs no installation.
    pub(crate) fn remove_unnecessary_packages<'l: 'a>(
        &mut self,
        get_locked: &impl Fn(&str) -> Option<LockedDependencyInfo<'l>>,
        env: &'env impl PackageCollection,
    ) {
        // Stop using the packages that got back to the installed version.
        // Requesting the installed version means reinstalling it, so we keep that.
        let reverted = (self.dependencies.iter())
            .filter(|&(name, info)| {
                !self.requested.contains(name)
                    && (info.using).is_some_and(|using| info.installed == Some(using.version()))
            })
            .map(|(&name, _)| name)
            .collect::<Vec<_>>();

        for name in reverted {
            self.stop_using(name, get_locked, env);
        }

        // Collect necessary packages: requested packages, dependencies of the project,
        // unlocked packages, and the packages required by necessary packages.
        // Installed packages are not always necessary since they can be unused after this.
        let necessary = self.collect_required(
            (self.requested.iter().copied())
                .chain(self.dependencies[""].requirements.keys().copied())
                .chain(self.unlocked_names.iter().copied()),
        );

        // Stop using the other packages since nobody requires them.
        let unnecessary = (self.dependencies.iter())
            .filter(|&(name, info)| info.using.is_some() && !necessary.contains(name))
            .map(|(&name, _)| name)
            .collect::<Vec<_>>();

        for name in unnecessary {
            log::debug!("package {name} is not necessary anymore");
            self.stop_using(name, get_locked, env);
        }
    }

    /// Collects the packages required by the `roots`. The result includes the `roots`.
    fn collect_required(&self, roots: impl Iterator<Item = &'a str>) -> HashSet<&'a str> {
        let mut required = HashSet::new();
        let mut queue = roots.collect::<VecDeque<_>>();

        while let Some(name) = queue.pop_front() {
            if !required.insert(name) {
                continue;
            }
            let Some(info) = self.dependencies.get(name) else {
                continue;
            };
            if info.is_legacy() {
                // modern packages are used instead of the legacy package
                queue.extend(&info.modern_packages);
            } else {
                queue.extend(&info.dependencies);
            }
        }

        required
    }

    /// Stops using the new version of the package, and gets back to as it was installed.
    fn stop_using<'l: 'a>(
        &mut self,
        name: &'a str,
        get_locked: &impl Fn(&str) -> Option<LockedDependencyInfo<'l>>,
        env: &'env impl PackageCollection,
    ) {
        let info = self.dependencies.get_mut(name).unwrap();
        info.using = None;
        info.current = info.installed;
        let dependencies = std::mem::take(&mut info.dependencies);
        let legacy_packages = std::mem::take(&mut info.legacy_packages);

        // The version is not used so its requirements must not be reported as conflicts.
        for dependency in dependencies {
            let dependency = self.dependencies.get_mut(dependency).unwrap();
            dependency.requirements.remove(name);
        }
        for legacy in legacy_packages.0 {
            let legacy = self.dependencies.get_mut(legacy.as_ref()).unwrap();
            legacy.modern_packages.remove(name);
        }

        if let Some(locked) = get_locked(name) {
            self.add_locked_dependency(locked, env);
        }
    }

    /// Finds the installed packages that become unused, and forgets their requirements.
    ///
    /// The package is unused if nobody requires it anymore, and it was required by
    /// the packages we replace or remove. We never remove other packages nobody requires.
    /// Such packages exist if the manifest is inconsistent, and they might be necessary.
    /// We keep them as they are, so the packages they require are not unused.
    pub(crate) fn remove_unused_packages<'l: 'a>(
        &mut self,
        get_locked: &impl Fn(&str) -> Option<LockedDependencyInfo<'l>>,
    ) {
        // Collect the packages that were required by the packages we replace or remove.
        let mut removable = HashSet::<&'a str>::new();
        let mut queue = (self.dependencies.iter())
            .filter(|(_, info)| info.using.is_some() || info.is_legacy())
            .map(|(&name, _)| name)
            .collect::<VecDeque<_>>();

        while let Some(name) = queue.pop_front() {
            if let Some(locked) = get_locked(name)
                && removable.insert(name)
            {
                queue.extend(locked.dependencies().keys().map(|x| x.as_ref()));
            }
        }

        // Collect the packages in use: dependencies of the project, unlocked packages,
        // the packages we install, the installed packages we never remove,
        // and the packages required by them.
        let in_use = self.collect_required(
            (self.dependencies[""].requirements.keys().copied())
                .chain(self.unlocked_names.iter().copied())
                .chain(
                    (self.dependencies.iter())
                        .filter(|&(name, info)| {
                            info.using.is_some()
                                || (info.installed.is_some() && !removable.contains(name))
                        })
                        .map(|(&name, _)| name),
                ),
        );

        // The package we replace or remove are not unused packages.
        let unused = (removable.into_iter())
            .filter(|name| !in_use.contains(name))
            .filter(|name| !self.dependencies[name].is_legacy())
            .collect::<Vec<_>>();

        for name in unused {
            log::debug!("package {name} is unused");

            // The package will be removed so its requirements must not be reported as conflicts.
            let info = self.dependencies.get_mut(name).unwrap();
            info.current = None;
            for dependency in std::mem::take(&mut info.dependencies) {
                let dependency = self.dependencies.get_mut(dependency).unwrap();
                dependency.requirements.remove(name);
            }

            self.unused.push(name);
        }
    }
}

impl<'env> ResolutionContext<'env, '_> {
    pub(crate) fn build_result(self) -> PackageResolutionResult<'env> {
        let mut conflicts = HashMap::<Box<str>, Vec<Box<str>>>::new();
        for (&name, info) in &self.dependencies {
            if !info.is_legacy()
                && info.touched
                && let Some(version) = &info.current
            {
                let conflicts_with_this = info
                    .requirements
                    .iter()
                    .filter(|&(&source, _)| self.is_active_source(source))
                    .filter(|(_, range)| {
                        !range.match_pre(
                            version,
                            PrereleaseAcceptance::allow_or_minimum(
                                info.allow_pre || self.allow_prerelease,
                            ),
                        )
                    })
                    .map(|(source, _)| *source)
                    .collect::<Vec<_>>();

                if !conflicts_with_this.is_empty()
                    && (info.using.is_some()
                        || conflicts_with_this
                            .iter()
                            .any(|x| self.dependencies[*x].using.is_some()))
                {
                    let vec = conflicts.entry(name.into()).or_default();
                    for source in conflicts_with_this {
                        vec.push(source.into())
                    }
                }
            }
        }

        let found_legacy_packages = self
            .dependencies
            .iter()
            .filter(|(_, info)| info.is_legacy())
            .map(|(&name, _)| name.into())
            .collect();

        let unused_packages = self.unused.iter().map(|&name| name.into()).collect();

        let new_packages = self
            .dependencies
            .into_values()
            .filter(|info| !info.is_legacy())
            .filter_map(|x| x.using)
            .collect();

        PackageResolutionResult {
            new_packages,
            conflicts,
            found_legacy_packages,
            unused_packages,
        }
    }
}

pub struct PackageResolutionResult<'env> {
    pub new_packages: Vec<PackageInfo<'env>>,
    // conflict dependency -> conflicting package[])
    pub conflicts: HashMap<Box<str>, Vec<Box<str>>>,
    // list of names of legacy packages we found
    pub found_legacy_packages: Vec<Box<str>>,
    // list of names of installed packages that become unused.
    // This is always empty unless `remove_unused` is specified.
    pub unused_packages: Vec<Box<str>>,
}

pub struct MissingDependencies {
    pub dependencies: HashMap<Box<str>, VersionRange>,
}

impl MissingDependencies {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.dependencies.is_empty()
    }

    pub fn add(&mut self, dependency: &str, range: &VersionRange) {
        match self.dependencies.entry(dependency.into()) {
            Entry::Occupied(mut e) => {
                e.insert(range.intersect(e.get()));
            }
            Entry::Vacant(e) => {
                e.insert(range.clone());
            }
        }
    }

    pub fn into_vec(self) -> Vec<(Box<str>, VersionRange)> {
        self.dependencies.into_iter().collect()
    }
}

impl<'env, 'a> ResolutionContext<'env, 'a>
where
    'env: 'a,
{
    /// Selects the version of the `dependency` for the `requirer`,
    /// and adds it to the queue if we have to install it.
    fn add_dependency(
        &mut self,
        requirer: &Requirer<'a>,
        dependency: &'a str,
        range: &'a VersionRange,
        env: &'env impl PackageCollection,
        unity_version: Option<UnityVersion>,
    ) {
        let name = requirer.name;
        log::debug!("processing package {name}: dependency {dependency} version {range}");

        if !self.should_add_package(dependency, range) {
            return;
        }

        let prerelease = if self.allow_prerelease {
            // prerelease is allowed, so we find the best match
            &[PrereleaseAcceptance::Allow]
        } else {
            requirer.prerelease
        };
        let find = |ranges: &[&VersionRange]| -> Option<PackageInfo<'env>> {
            // We prefer the versions compatible with the unity of the project,
            // but we use incompatible versions if there are no compatible versions.
            [unity_version, None].into_iter().find_map(|unity_version| {
                prerelease.iter().find_map(|&prerelease| {
                    env.find_package_by_name(
                        dependency,
                        VersionSelector::ranges_for(unity_version, ranges, prerelease),
                    )
                })
            })
        };

        // Prefer the version that satisfies the requirements from all packages we know,
        // not to make a conflict when some version satisfies everyone.
        // If there is no such version, try without the requirements the user is already ignoring,
        // and finally, the requirement of this package wins.
        let allow_downgrade = requirer.allow_downgrade;
        let not_downgrade = |found: &PackageInfo<'env>| {
            allow_downgrade || !self.is_downgrade(dependency, found.version())
        };
        let found = (find(&self.requirements_for(dependency)).filter(not_downgrade))
            .or_else(|| {
                find(&self.unignored_requirements_for(dependency, name)).filter(not_downgrade)
            })
            .or_else(|| find(&[range]));

        if let Some(found) = found {
            if !allow_downgrade && self.is_downgrade(dependency, found.version()) {
                log::debug!(
                    "processing package {name}: dependency {dependency} version {range}: \
                    keeping installed version since {} is downgrade",
                    found.version()
                );
            } else {
                self.pending_queue.add_pending_package(found);
            }
        } else {
            self.missing.push((name, dependency, range));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn collect_adding_packages<'a, 'env>(
    dependencies: impl Iterator<Item = (&'a str, &'a DependencyRange)>,
    locked_dependencies: impl Iterator<Item = LockedDependencyInfo<'a>>,
    unlocked_packages: impl Iterator<Item = &'a (Box<str>, Option<PackageManifest>)>,
    get_locked: impl Fn(&str) -> Option<LockedDependencyInfo<'a>>,
    unity_version: Option<UnityVersion>,
    env: &'env impl PackageCollection,
    requests: Vec<Request<'env, 'a>>,
    allow_prerelease: bool,
    remove_unused: bool,
    missing_dependencies: &mut MissingDependencies,
) -> PackageResolutionResult<'env> {
    let mut packages = vec![];
    let mut keep_locked = vec![];
    let mut range_requests = vec![];
    for request in requests {
        match request {
            Request::Package(package) => packages.push(package),
            Request::KeepLocked(name) => keep_locked.push(name),
            Request::Range {
                name,
                range,
                prerelease,
            } => range_requests.push((name, range, prerelease)),
        }
    }

    let mut context = ResolutionContext::<'env, '_>::new(allow_prerelease, packages);
    context.requested.extend(keep_locked);

    // first, add dependencies
    let root_dependencies = dependencies
        .into_iter()
        .map(|(name, dependency)| {
            let (range, mut allow_pre);

            if let Some(mut min_ver) = dependency.as_single_version() {
                allow_pre = min_ver.is_pre();
                if let Some(locked) = get_locked(name) {
                    allow_pre |= !locked.version().pre.is_empty();
                    if locked.version() < &min_ver {
                        min_ver = locked.version().clone();
                    }
                }
                range = VersionRange::same_or_later(min_ver);
            } else {
                range = dependency.as_range();
                allow_pre = range.contains_pre();
            }

            (name, range, allow_pre)
        })
        .collect::<Vec<_>>();

    for (name, range, allow_pre) in &root_dependencies {
        context.add_root_dependency(name, range, *allow_pre);
    }

    // then, add locked dependencies info
    for locked in locked_dependencies {
        context.add_locked_dependency(locked, env);
    }

    // add unlocked packages
    for (unlocked_name, unlocked_manifest) in unlocked_packages {
        context.add_unlocked_name(unlocked_name.as_ref());
        if let Some(unlocked_package) = unlocked_manifest {
            context.add_unlocked_name(unlocked_package.name());
            context.add_unlocked_manifest(unlocked_package);
        }
    }

    context.mark_ignored_requirements();

    // Select the versions for the requests with ranges like dependencies of installing packages.
    for (name, range, prerelease) in &range_requests {
        let requirer = Requirer {
            name: "",
            prerelease,
            allow_downgrade: false,
        };
        context.add_dependency(&requirer, name, range, env, unity_version);
    }

    while let Some((x, force)) = context.pending_queue.next_package() {
        log::debug!("processing package {} version {}", x.name(), x.version());
        let vpm_dependencies = &x.vpm_dependencies();
        let requirer = Requirer {
            name: x.name(),
            prerelease: if x.version().is_pre() {
                // if the package is prerelease, allow prerelease, but prefer stable
                &[
                    PrereleaseAcceptance::Deny,
                    PrereleaseAcceptance::Minimum,
                    PrereleaseAcceptance::Allow,
                ]
            } else {
                // if the package is stable, prefer stable, and allow minimum
                &[PrereleaseAcceptance::Deny, PrereleaseAcceptance::Minimum]
            },
            allow_downgrade: context.is_downgrade(x.name(), x.version()),
        };

        if context.add_package(x, force) {
            // add new dependencies
            for (dependency, range) in vpm_dependencies.iter() {
                context.add_dependency(&requirer, dependency, range, env, unity_version);
            }
        }
    }

    context.remove_unnecessary_packages(&get_locked, env);
    if remove_unused {
        context.remove_unused_packages(&get_locked);
    }

    // Report only the missing dependencies of the packages we actually install.
    for &(requirer, dependency, range) in &context.missing {
        let installing = (context.dependencies.get(requirer))
            .is_some_and(|info| info.using.is_some() && !info.is_legacy());
        if requirer.is_empty() || installing {
            missing_dependencies.add(dependency, range);
        }
    }

    context.build_result()
}

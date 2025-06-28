use crate::traits::PackageCollection;
use crate::unity_project::LockedDependencyInfo;
use crate::version::{DependencyRange, PrereleaseAcceptance, UnityVersion, Version, VersionRange};
use crate::{PackageInfo, PackageManifest, VersionSelector};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet, VecDeque};

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
        self.pending_queue.pop_back().map(|x| (x, force))
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
    // "" key for root dependencies
    requirements: HashMap<&'a str, &'a VersionRange>,
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
            requirements,
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
        self.dependencies = dependencies;
    }
}

impl<'env> ResolutionContext<'env, '_> {
    fn new(allow_prerelease: bool, packages: Vec<PackageInfo<'env>>) -> Self {
        let mut this = Self {
            dependencies: HashMap::new(),
            pending_queue: PackageQueue::new(packages),
            allow_prerelease,
            unlocked_names: HashSet::new(),
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
                    .filter(|&(&source, _)| {
                        self.dependencies
                            .get(source)
                            .map(|x| !x.is_legacy())
                            .unwrap_or_default()
                    })
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
        }
    }
}

pub struct PackageResolutionResult<'env> {
    pub new_packages: Vec<PackageInfo<'env>>,
    // conflict dependency -> conflicting package[])
    pub conflicts: HashMap<Box<str>, Vec<Box<str>>>,
    // list of names of legacy packages we found
    pub found_legacy_packages: Vec<Box<str>>,
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn collect_adding_packages<'a, 'env>(
    dependencies: impl Iterator<Item = (&'a str, &'a DependencyRange)>,
    locked_dependencies: impl Iterator<Item = LockedDependencyInfo<'a>>,
    unlocked_packages: impl Iterator<Item = &'a (Box<str>, Option<PackageManifest>)>,
    get_locked: impl Fn(&str) -> Option<LockedDependencyInfo<'a>>,
    unity_version: Option<UnityVersion>,
    env: &'env impl PackageCollection,
    packages: Vec<PackageInfo<'env>>,
    allow_prerelease: bool,
    missing_dependencies: &mut MissingDependencies,
) -> PackageResolutionResult<'env> {
    let mut context = ResolutionContext::<'env, '_>::new(allow_prerelease, packages);

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

    while let Some((x, force)) = context.pending_queue.next_package() {
        log::debug!("processing package {} version {}", x.name(), x.version());
        let name = x.name();
        let vpm_dependencies = &x.vpm_dependencies();

        if context.add_package(x, force) {
            // add new dependencies
            for (dependency, range) in vpm_dependencies.iter() {
                log::debug!("processing package {name}: dependency {dependency} version {range}");

                if context.should_add_package(dependency, range) {
                    fn get_package<'env>(
                        env: &'env impl PackageCollection,
                        dependency: &str,
                        unity_version: Option<UnityVersion>,
                        range: &VersionRange,
                        allow_prerelease: PrereleaseAcceptance,
                    ) -> Option<PackageInfo<'env>> {
                        env.find_package_by_name(
                            dependency,
                            VersionSelector::range_for(unity_version, range, allow_prerelease),
                        )
                    }

                    struct PackageFinder<'env, 'a, C: PackageCollection> {
                        dependency: &'a str,
                        env: &'env C,
                        range: &'env VersionRange,
                    }

                    impl<'env, C: PackageCollection> PackageFinder<'env, '_, C> {
                        fn find(
                            &self,
                            unity_version: Option<UnityVersion>,
                            allow_prerelease: PrereleaseAcceptance,
                        ) -> Option<PackageInfo<'env>> {
                            get_package(
                                self.env,
                                self.dependency,
                                unity_version,
                                self.range,
                                allow_prerelease,
                            )
                        }
                    }

                    let finder = PackageFinder {
                        dependency,
                        env,
                        range,
                    };

                    let found;
                    if allow_prerelease {
                        // prerelease is allowed, so we find the best match
                        found = (finder.find(unity_version, PrereleaseAcceptance::Allow))
                            .or_else(|| finder.find(None, PrereleaseAcceptance::Allow));
                    } else if x.version().is_pre() {
                        // if the package is prerelease, allow prerelease, but prefer stable
                        found = (finder.find(unity_version, PrereleaseAcceptance::Deny))
                            .or_else(|| finder.find(unity_version, PrereleaseAcceptance::Minimum))
                            .or_else(|| finder.find(unity_version, PrereleaseAcceptance::Allow))
                            .or_else(|| finder.find(None, PrereleaseAcceptance::Deny))
                            .or_else(|| finder.find(None, PrereleaseAcceptance::Minimum))
                            .or_else(|| finder.find(None, PrereleaseAcceptance::Allow));
                    } else {
                        // if the package is stable, prefer stable, and allow minimum
                        found = (finder.find(unity_version, PrereleaseAcceptance::Deny))
                            .or_else(|| finder.find(unity_version, PrereleaseAcceptance::Minimum))
                            .or_else(|| finder.find(None, PrereleaseAcceptance::Deny))
                            .or_else(|| finder.find(None, PrereleaseAcceptance::Minimum));
                    }

                    if let Some(found) = found {
                        context.pending_queue.add_pending_package(found);
                    } else {
                        missing_dependencies.add(dependency, range);
                    }
                }
            }
        }
    }

    context.build_result()
}

use std::collections::VecDeque;
use super::*;

struct PackageQueue<'a> {
    pending_queue: VecDeque<PackageInfo<'a>>,
}

impl<'a> PackageQueue<'a> {
    fn new(packages: Vec<PackageInfo<'a>>) -> Self {
        Self {
            pending_queue: VecDeque::from_iter(packages),
        }
    }

    pub(crate) fn next_package(&mut self) -> Option<PackageInfo<'a>> {
        self.pending_queue.pop_back()
    }

    fn find_pending_package(&self, name: &str) -> Option<&PackageInfo<'a>> {
        self.pending_queue.iter().find(|x| x.name() == name)
    }

    pub(crate) fn add_pending_package(&mut self, package: PackageInfo<'a>) {
        self.pending_queue.retain(|x| x.name() != package.name());
        self.pending_queue.push_back(package);
    }
}

struct ResolutionContext<'env, 'a> where 'env: 'a {
    allow_prerelease: bool,
    pub pending_queue: PackageQueue<'env>,
    dependencies: HashMap<&'a str, DependencyInfo<'env, 'a>>,
}

struct Legacy<'env>(&'env Vec<String>);

impl <'env> Default for Legacy<'env> {
    fn default() -> Self {
        static VEC: Vec<String> = Vec::new();
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
    legacy_packages: Legacy<'env>,

    allow_pre: bool,
    touched: bool,
}

impl <'env, 'a> DependencyInfo<'env, 'a> where 'env: 'a {
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
        self.modern_packages.len() != 0
    }

    pub(crate) fn set_using_info(&mut self, version: &'a Version, dependencies: HashSet<&'a str>) {
        self.allow_pre |= !version.pre.is_empty();
        self.current = Some(version);
        self.dependencies = dependencies;
    }
}

impl<'env, 'a> ResolutionContext<'env, 'a> {
    fn new(allow_prerelease: bool, packages: Vec<PackageInfo<'env>>) -> Self {
        let mut this = Self {
            dependencies: HashMap::new(),
            pending_queue: PackageQueue::new(packages),
            allow_prerelease
        };

        for pkg in &this.pending_queue.pending_queue {
            this.dependencies.entry(pkg.name()).or_default().allow_pre = true;
        }

        this
    }
}

impl<'env, 'a> ResolutionContext<'env, 'a> where 'env: 'a {
    pub(crate) fn add_root_dependency(&mut self, name: &'a str, range: &'a VersionRange, allow_pre: bool) {
        self.dependencies.insert(name, DependencyInfo::new_dependency(range, allow_pre));
    }

    pub(crate) fn add_locked_dependency(&mut self, name: &'a String, locked: &'a VpmLockedDependency, env: &'env Environment) {
        let info = self.dependencies.entry(name).or_default();
        info.set_using_info(&locked.version, locked.dependencies.keys().map(|x| x.as_str()).collect());


        if let Some(pkg) = env.find_package_by_name(name, PackageSelector::specific_version(&locked.version)) {
            info.legacy_packages = Legacy(pkg.legacy_packages());

            for legacy in pkg.legacy_packages() {
                self.dependencies.entry(legacy).or_default().modern_packages.insert(name);
            }
        }

        for (dependency, range) in &locked.dependencies {
            self.dependencies.entry(dependency).or_default().requirements.insert(name, range);
        }
    }

    pub(crate) fn add_package(&mut self, package: PackageInfo<'env>) -> bool {
        let entry = self.dependencies.entry(package.name()).or_default();

        if entry.is_legacy() {
            return false;
        }

        let vpm_dependencies = &package.vpm_dependencies();
        let legacy_packages = package.legacy_packages();
        let name = package.name();

        entry.touched = true;
        entry.current = Some(&package.version());
        entry.using = Some(package);

        let old_dependencies = std::mem::replace(&mut entry.dependencies, vpm_dependencies.keys().map(|x| x.as_str()).collect());
        let old_legacy_packages = std::mem::replace(&mut entry.legacy_packages, Legacy(legacy_packages));

        // region process dependencies
        // remove previous dependencies if exists
        for dep in &old_dependencies {
            self.dependencies.get_mut(*dep).unwrap().remove_range(name);
        }
        for (dependency, range) in vpm_dependencies.iter() {
            self.dependencies.entry(dependency).or_default().add_range(name, range)
        }
        // endregion

        // region process modern packages
        for dep in old_legacy_packages.0 {
            self.dependencies.get_mut(dep.as_str()).unwrap().remove_modern_package(name);
        }
        for legacy in legacy_packages {
            self.dependencies.entry(legacy).or_default().add_modern_package(name)
        }
        // endregion

        true
    }

    pub(crate) fn should_add_package(&self, name: &'a str, range: &'a VersionRange) -> bool {
        let entry = self.dependencies.get(name).unwrap();
        
        if entry.is_legacy() {
            return false
        }

        let mut install = true;
        let allow_prerelease = entry.allow_pre || self.allow_prerelease;

        if let Some(pending) = self.pending_queue.find_pending_package(name) {
            if range.match_pre(&pending.version(), allow_prerelease) {
                // if installing version is good, no need to reinstall
                install = false;
                log::debug!("processing package {name}: dependency {name} version {range}: pending matches");
            }
        } else {
            // if already installed version is good, no need to reinstall
            if let Some(version) = &entry.current {
                if range.match_pre(version, allow_prerelease) {
                    log::debug!("processing package {name}: dependency {name} version {range}: existing matches");
                    install = false;
                }
            }
        }

        return install;
    }
}

impl<'env, 'a> ResolutionContext<'env, 'a> {
    pub(crate) fn build_result(self) -> PackageResolutionResult<'env> {
        let mut conflicts = HashMap::<String, Vec<String>>::new();
        for (&name, info) in &self.dependencies {
            if !info.is_legacy() && info.touched {
                if let Some(version) = &info.current {
                    for (source, range) in &info.requirements {
                        if !range.match_pre(version, info.allow_pre || self.allow_prerelease) {
                            conflicts.entry(name.to_owned()).or_default().push((*source).to_owned());
                        }
                    }
                }
            }
        }

        let found_legacy_packages = self.dependencies
            .iter()
            .filter(|(_, info)| info.is_legacy())
            .map(|(&name, _)| name.to_owned())
            .collect();

        let new_packages= self.dependencies
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
    pub conflicts: HashMap<String, Vec<String>>,
    // list of names of legacy packages we found
    pub found_legacy_packages: Vec<String>,
}

pub fn collect_adding_packages<'env>(
    dependencies: &IndexMap<String, VpmDependency>,
    locked_dependencies: &IndexMap<String, VpmLockedDependency>,
    unity_version: Option<UnityVersion>,
    env: &'env Environment,
    packages: Vec<PackageInfo<'env>>,
    allow_prerelease: bool,
) -> Result<PackageResolutionResult<'env>, AddPackageErr> {
    let mut context = ResolutionContext::<'env, '_>::new(allow_prerelease, packages);

    // first, add dependencies
    let mut root_dependencies = Vec::with_capacity(dependencies.len());

    for (name, dependency) in dependencies {
        let (range, mut allow_pre);

        if let Some(mut min_ver) = dependency.version.as_single_version() {
            allow_pre = min_ver.is_pre();
            if let Some(locked) = locked_dependencies.get(name) {
                allow_pre |= !locked.version.pre.is_empty();
                if &locked.version < &min_ver {
                    min_ver = locked.version.clone();
                }
            }
            range = VersionRange::same_or_later(min_ver);
        } else {
            range = dependency.version.as_range();
            allow_pre = range.contains_pre();
        }

        root_dependencies.push((name, range, allow_pre));
    }

    for (name, range, allow_pre) in &root_dependencies {
        context.add_root_dependency(name, range, *allow_pre);
    }

    // then, add locked dependencies info
    for (source, locked) in locked_dependencies {
        context.add_locked_dependency(source, locked, env);
    }

    while let Some(x) = context.pending_queue.next_package() {
        log::debug!("processing package {} version {}", x.name(), x.version());
        let name = x.name();
        let vpm_dependencies = &x.vpm_dependencies();

        if context.add_package(x) {
            // add new dependencies
            for (dependency, range) in vpm_dependencies.iter() {
                log::debug!("processing package {name}: dependency {dependency} version {range}");

                if context.should_add_package(dependency, range) {
                    let found = env
                        .find_package_by_name(dependency, PackageSelector::range_for(unity_version, range))
                        .or_else(||env.find_package_by_name(dependency, PackageSelector::range_for(None, range)))
                        .ok_or_else(|| AddPackageErr::DependencyNotFound {
                            dependency_name: dependency.clone(),
                        })?;

                    // remove existing if existing
                    context.pending_queue.add_pending_package(found);
                }
            }
        }
    }

    Ok(context.build_result())
}

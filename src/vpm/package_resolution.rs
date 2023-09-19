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

#[derive(Default)]
struct DependencyInfo<'env, 'a> {
    using: Option<PackageInfo<'env>>,
    current: Option<&'a Version>,
    // "" key for root dependencies
    requirements: HashMap<&'a str, &'a VersionRange>,
    dependencies: HashSet<&'a str>,
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

    pub(crate) fn set_using_info(&mut self, version: &'a Version, dependencies: HashSet<&'a str>) {
        self.allow_pre |= !version.pre.is_empty();
        self.current = Some(version);
        self.dependencies = dependencies;
    }
}

impl<'env, 'a> ResolutionContext<'env, 'a> {
    fn new(allow_prerelease: bool, packages: Vec<PackageInfo<'env>>) -> Self {
        Self {
            dependencies: HashMap::new(),
            pending_queue: PackageQueue::new(packages),
            allow_prerelease
        }
    }
}

impl<'env, 'a> ResolutionContext<'env, 'a> where 'env: 'a {
    pub(crate) fn add_root_dependency(&mut self, name: &'a str, range: &'a VersionRange, allow_pre: bool) {
        self.dependencies.insert(name, DependencyInfo::new_dependency(range, allow_pre));
    }

    pub(crate) fn add_locked_dependency(&mut self, name: &'a String, locked: &'a VpmLockedDependency, _env: &'env Environment) {
        self.dependencies.entry(name).or_default()
            .set_using_info(&locked.version, locked.dependencies.keys().map(|x| x.as_str()).collect());

        for (dependency, range) in &locked.dependencies {
            self.dependencies.entry(dependency).or_default().requirements.insert(name, range);
        }
    }

    pub(crate) fn add_package(&mut self, package: PackageInfo<'env>) {
        let entry = self.dependencies.entry(package.name()).or_default();

        let vpm_dependencies = &package.vpm_dependencies();
        let dependencies = vpm_dependencies.keys().map(|x| x.as_str()).collect();

        entry.touched = true;
        entry.current = Some(&package.version());
        entry.using = Some(package);
        let old_dependencies = std::mem::replace(&mut entry.dependencies, dependencies);

        // remove previous dependencies if exists
        for dep in &old_dependencies {
            self.dependencies.get_mut(*dep).unwrap().remove_range(dep);
        }

        for (dependency, range) in vpm_dependencies.iter() {
            self.dependencies.entry(dependency).or_default().add_range(dependency, range)
        }
    }

    pub(crate) fn should_add_package(&self, name: &'a str, range: &'a VersionRange) -> bool {
        let entry = self.dependencies.get(name).unwrap();
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
    pub(crate) fn find_conflicts(&self) -> Result<(), AddPackageErr> {
        for (name, info) in &self.dependencies {
            if info.touched {
                if let Some(version) = &info.current {
                    for (mut source, range) in &info.requirements {
                        if !range.match_pre(version, info.allow_pre || self.allow_prerelease) {
                            if source == &"" {
                                source = &"dependencies";
                            }
                            return Err(AddPackageErr::ConflictWithDependencies {
                                conflict: (*name).to_owned(),
                                dependency_name: (*source).to_owned(),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub(crate) fn installing_packages(self) -> Vec<PackageInfo<'env>> {
        self.dependencies
            .into_values()
            .filter_map(|x| x.using)
            .collect()
    }
}

pub fn collect_adding_packages<'env>(
    unity: &UnityProject,
    env: &'env Environment,
    packages: Vec<PackageInfo<'env>>,
    allow_prerelease: bool,
) -> Result<Vec<PackageInfo<'env>>, AddPackageErr> {
    let mut context = ResolutionContext::<'env, '_>::new(allow_prerelease, packages);

    // first, add dependencies
    let mut root_dependencies = Vec::with_capacity(unity.manifest.dependencies().len());

    for (name, dependency) in unity.manifest.dependencies() {
        let mut min_ver = &dependency.version;
        let mut allow_pre = !dependency.version.pre.is_empty();

        if let Some(locked) = unity.manifest.locked().get(name) {
            allow_pre |= !locked.version.pre.is_empty();
            if &locked.version < min_ver {
                min_ver = &locked.version;
            }
        }

        root_dependencies.push((name, VersionRange::same_or_later(min_ver.clone()), allow_pre));
    }

    for (name, range, allow_pre) in &root_dependencies {
        context.add_root_dependency(name, range, *allow_pre);
    }

    // then, add locked dependencies info
    for (source, locked) in unity.manifest.locked() {
        context.add_locked_dependency(source, locked, env);
    }

    while let Some(x) = context.pending_queue.next_package() {
        log::debug!("processing package {} version {}", x.name(), x.version());
        let name = x.name();
        let vpm_dependencies = &x.vpm_dependencies();

        context.add_package(x);

        // add new dependencies
        for (dependency, range) in vpm_dependencies.iter() {
            log::debug!("processing package {name}: dependency {dependency} version {range}");

            if context.should_add_package(dependency, range) {
                let found = env
                    .find_package_by_name(dependency, VersionSelector::Range(range))
                    .ok_or_else(|| AddPackageErr::DependencyNotFound {
                        dependency_name: dependency.clone(),
                    })?;

                // remove existing if existing
                context.pending_queue.add_pending_package(found);
            }
        }
    }

    // finally, check for conflict.
    context.find_conflicts()?;

    Ok(context.installing_packages())
}

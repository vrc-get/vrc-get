use super::*;

pub fn collect_adding_packages<'env>(
    unity: &UnityProject,
    env: &'env Environment,
    packages: Vec<PackageInfo<'env>>,
    allow_prerelease: bool,
) -> Result<Vec<PackageInfo<'env>>, AddPackageErr> {
    #[derive(Default)]
    struct DependencyInfo<'env, 'a> {
        using: Option<PackageInfo<'env>>,
        current: Option<&'a Version>,
        // "" key for root dependencies
        requirements: HashMap<&'a str, &'a VersionRange>,
        dependencies: HashSet<&'a str>,
        allow_pre: bool,
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
            }
        }

        fn add_range(&mut self, source: &'a str, range: &'a VersionRange) {
            self.requirements.insert(source, range);
        }

        fn remove_range(&mut self, source: &str) {
            self.requirements.remove(source);
        }

        pub(crate) fn set_using_info(&mut self, version: &'a Version, dependencies: HashSet<&'a str>) {
            self.allow_pre |= !version.pre.is_empty();
            self.current = Some(version);
            self.dependencies = dependencies;
        }

        pub(crate) fn set_package(&mut self, new_pkg: PackageInfo<'env>) -> HashSet<&'a str> {
            let mut dependencies = new_pkg.vpm_dependencies()
                .keys().map(|x| x.as_str()).collect();

            self.current = Some(&new_pkg.version());
            std::mem::swap(&mut self.dependencies, &mut dependencies);
            self.using = Some(new_pkg);

            // using is save
            return dependencies
        }
    }

    let mut dependencies = HashMap::<&str, _>::new();

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
        dependencies.insert(name, DependencyInfo::new_dependency(range, *allow_pre));
    }

    // then, add locked dependencies info
    for (source, locked) in unity.manifest.locked() {
        dependencies.entry(source).or_default()
            .set_using_info(&locked.version, locked.dependencies.keys().map(|x| x.as_str()).collect());

        for (dependency, range) in &locked.dependencies {
            dependencies.entry(dependency).or_default()
                .add_range(source, range)
        }
    }

    let mut packages = std::collections::VecDeque::from_iter(packages);

    while let Some(x) = packages.pop_front() {
        log::debug!("processing package {} version {}", x.name(), x.version());
        let name = x.name();
        let vpm_dependencies = &x.vpm_dependencies();
        let entry = dependencies.entry(x.name()).or_default();
        let old_dependencies = entry.set_package(x);

        // remove previous dependencies if exists
        for dep in &old_dependencies {
            dependencies.get_mut(*dep).unwrap().remove_range(dep);
        }

        // add new dependencies
        for (dependency, range) in vpm_dependencies.iter() {
            log::debug!("processing package {name}: dependency {dependency} version {range}");
            let entry = dependencies.entry(dependency).or_default();
            let mut install = true;
            let allow_prerelease = entry.allow_pre || allow_prerelease;

            if packages.iter().any(|x| x.name() == dependency && range.match_pre(&x.version(), allow_prerelease)) {
                // if installing version is good, no need to reinstall
                install = false;
                log::debug!("processing package {name}: dependency {dependency} version {range}: pending matches");
            } else {
                // if already installed version is good, no need to reinstall
                if let Some(version) = &entry.current {
                    if range.match_pre(version, allow_prerelease) {
                        log::debug!("processing package {name}: dependency {dependency} version {range}: existing matches");
                        install = false;
                    }
                }
            }

            entry.add_range(name, range);

            if install {
                let found = env
                    .find_package_by_name(dependency, VersionSelector::Range(range))
                    .ok_or_else(|| AddPackageErr::DependencyNotFound {
                        dependency_name: dependency.clone(),
                    })?;

                // remove existing if existing
                packages.retain(|x| x.name() != dependency);
                packages.push_back(found);
            }
        }
    }

    // finally, check for conflict.
    for (name, info) in &dependencies {
        if let Some(version) = &info.current {
            for (mut source, range) in &info.requirements {
                if !range.match_pre(version, info.allow_pre || allow_prerelease) {
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

    Ok(dependencies
        .into_values()
        .filter_map(|x| x.using)
        .collect())
}

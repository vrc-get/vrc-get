use std::path::PathBuf;
use vrc_get_vpm::{PackageInfo, PackageJson, VersionSelector};

pub(crate) struct PackageCollection {
    packages: Vec<(PackageJson, PathBuf)>,
}

impl PackageCollection {
    pub fn new() -> Self {
        Self { packages: vec![] }
    }

    pub fn add(&mut self, path: impl Into<PathBuf>, json: PackageJson) {
        self.packages.push((json, path.into()));
    }
}

impl vrc_get_vpm::PackageCollection for PackageCollection {
    fn get_all_packages(&self) -> impl Iterator<Item = PackageInfo> {
        self.packages
            .iter()
            .map(|(json, path)| PackageInfo::local(json, path))
    }

    fn find_packages(&self, package: &str) -> impl Iterator<Item = PackageInfo> {
        self.get_all_packages()
            .filter(move |pkg| pkg.name() == package)
    }

    fn find_package_by_name(
        &self,
        name: &str,
        version: VersionSelector,
    ) -> Option<PackageInfo<'_>> {
        self.find_packages(name).find(|pkg| version.satisfies(pkg))
    }
}

use std::path::PathBuf;
use vrc_get_vpm::version::Version;
use vrc_get_vpm::{PackageCollection as _, PackageInfo, PackageManifest, VersionSelector};

pub struct PackageCollectionBuilder {
    packages: Vec<(PackageManifest, PathBuf)>,
}

impl PackageCollectionBuilder {
    pub fn new() -> Self {
        Self { packages: vec![] }
    }

    pub fn add(mut self, json: PackageManifest) -> PackageCollectionBuilder {
        let path = format!("Packages/{}/{}", json.name(), json.version());
        self.packages.push((json, path.into()));
        self
    }

    pub fn build(self) -> PackageCollection {
        PackageCollection {
            packages: self.packages,
        }
    }
}

pub struct PackageCollection {
    packages: Vec<(PackageManifest, PathBuf)>,
}

impl PackageCollection {
    pub fn get_package(&self, name: &str, version: Version) -> PackageInfo<'_> {
        self.find_package_by_name(name, VersionSelector::specific_version(&version))
            .unwrap()
    }
}

impl vrc_get_vpm::PackageCollection for PackageCollection {
    fn get_all_packages(&self) -> impl Iterator<Item = PackageInfo<'_>> {
        self.packages
            .iter()
            .map(|(json, path)| PackageInfo::local(json, path))
    }

    fn find_packages(&self, package: &str) -> impl Iterator<Item = PackageInfo<'_>> {
        self.get_all_packages()
            .filter(move |pkg| pkg.name() == package)
    }

    fn find_package_by_name(
        &self,
        name: &str,
        version: VersionSelector,
    ) -> Option<PackageInfo<'_>> {
        self.find_packages(name)
            .find(|pkg| version.satisfies(pkg.package_json()))
    }
}

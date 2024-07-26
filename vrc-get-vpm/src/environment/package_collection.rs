use std::path::PathBuf;

use crate::repository::LocalCachedRepository;
use crate::{PackageInfo, PackageManifest, VersionSelector};

/// A immutable structure that holds information about all the packages.
pub struct PackageCollection {
    pub(super) repositories: Vec<LocalCachedRepository>,
    pub(super) user_packages: Vec<(PathBuf, PackageManifest)>,
}

impl crate::PackageCollection for PackageCollection {
    fn get_curated_packages(
        &self,
        version_selector: VersionSelector,
    ) -> impl Iterator<Item = PackageInfo> {
        self.repositories
            .iter()
            .filter(|x| x.repo.id() == Some("com.vrchat.repos.curated"))
            .flat_map(move |repo| {
                repo.repo()
                    .get_packages()
                    .filter_map(move |x| x.get_latest(version_selector))
                    .map(|json| PackageInfo::remote(json, repo))
            })
    }

    fn get_all_packages(&self) -> impl Iterator<Item = PackageInfo> {
        let remote = self.repositories.iter().flat_map(|repo| {
            repo.repo
                .get_packages()
                .flat_map(|x| x.all_versions())
                .map(|pkg| PackageInfo::remote(pkg, repo))
        });
        let local = self
            .user_packages
            .iter()
            .map(|(path, json)| PackageInfo::local(json, path));

        remote.chain(local)
    }

    fn find_packages(&self, package: &str) -> impl Iterator<Item = PackageInfo> {
        let remote = self.repositories.iter().flat_map(|repo| {
            repo.repo
                .get_package(package)
                .into_iter()
                .flat_map(|x| x.all_versions().map(|pkg| PackageInfo::remote(pkg, repo)))
        });
        let local = self
            .user_packages
            .iter()
            .filter(move |(_, json)| json.name() == package)
            .map(|(path, json)| PackageInfo::local(json, path));

        remote.chain(local)
    }

    fn find_package_by_name(
        &self,
        package: &str,
        package_selector: VersionSelector,
    ) -> Option<PackageInfo> {
        let remote = self.repositories.iter().flat_map(|repo| {
            repo.repo
                .get_package(package)
                .into_iter()
                .flat_map(|pkg| pkg.get_latest(package_selector))
                .map(|pkg| PackageInfo::remote(pkg, repo))
        });

        let local = self
            .user_packages
            .iter()
            .filter(move |(_, json)| json.name() == package && package_selector.satisfies(json))
            .map(|(path, json)| PackageInfo::local(json, path));

        remote.chain(local).max_by_key(|x| x.version())
    }
}

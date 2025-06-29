use std::path::PathBuf;

use crate::PackageCollection as _;
use crate::environment::{RepoHolder, Settings, UserPackageCollection};
use crate::io::{DefaultEnvironmentIo, IoTrait};
use crate::repository::LocalCachedRepository;
use crate::{HttpClient, PackageInfo, PackageManifest, UserRepoSetting, VersionSelector, io};
use futures::prelude::*;
use itertools::Itertools;
use log::error;

/// A immutable structure that holds information about all the packages.
#[derive(Debug, Clone)]
pub struct PackageCollection {
    pub(super) repositories: RepoHolder,
    pub(super) user_packages: Vec<(PathBuf, PackageManifest)>,
}

impl PackageCollection {
    pub fn empty() -> Self {
        Self {
            repositories: RepoHolder::new(),
            user_packages: Vec::new(),
        }
    }

    pub async fn load_cache(settings: &Settings, io: &DefaultEnvironmentIo) -> io::Result<Self> {
        let (repositories, user_packages) = futures::try_join!(
            RepoHolder::load_cache(settings, io),
            UserPackageCollection::load(settings, io).map(Ok)
        )?;

        Ok(Self {
            repositories,
            user_packages: user_packages.into_packages(),
        })
    }

    pub async fn load(
        settings: &Settings,
        io: &DefaultEnvironmentIo,
        http: Option<&impl HttpClient>,
    ) -> io::Result<Self> {
        let (repositories, user_packages) = futures::try_join!(
            RepoHolder::load(settings, io, http),
            UserPackageCollection::load(settings, io).map(Ok)
        )?;

        Ok(Self {
            repositories,
            user_packages: user_packages.into_packages(),
        })
    }

    pub async fn update_cache(&mut self, io: &DefaultEnvironmentIo, http: &impl HttpClient) {
        self.repositories.update_cache(io, http).await
    }

    pub async fn remove_repositories(
        &mut self,
        remove_repos: &[UserRepoSetting],
        io: &DefaultEnvironmentIo,
    ) {
        for duplicated_repo in remove_repos {
            error!(
                "Duplicated repository id: {}",
                duplicated_repo.local_path().display()
            );
            io.remove_file(duplicated_repo.local_path()).await.ok();
            self.repositories.remove(duplicated_repo.local_path());
        }
    }
}

impl PackageCollection {
    pub fn get_remote(&self) -> impl Iterator<Item = &'_ LocalCachedRepository> {
        self.repositories.iter()
    }

    pub fn user_packages(&self) -> &[(PathBuf, PackageManifest)] {
        &self.user_packages
    }

    pub fn find_whole_all_packages(
        &self,
        version_selector: VersionSelector,
        filter: impl Fn(&PackageManifest) -> bool,
    ) -> Vec<PackageInfo<'_>> {
        self.get_all_packages()
            .filter(|x| version_selector.satisfies(x.package_json()))
            .into_group_map_by(|x| x.name())
            .values()
            .map(|versions| versions.iter().max_by_key(|x| x.version()).unwrap())
            .filter(|x| filter(x.package_json()))
            .copied()
            .collect()
    }
}

impl crate::PackageCollection for PackageCollection {
    fn get_curated_packages(
        &self,
        version_selector: VersionSelector,
    ) -> impl Iterator<Item = PackageInfo<'_>> {
        self.repositories
            .find_by_id("com.vrchat.repos.curated")
            .map(move |repo| {
                repo.repo()
                    .get_packages()
                    .filter_map(move |x| x.get_latest(version_selector))
                    .map(|json| PackageInfo::remote(json, repo))
            })
            .into_iter()
            .flatten()
    }

    fn get_all_packages(&self) -> impl Iterator<Item = PackageInfo<'_>> {
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

    fn find_packages(&self, package: &str) -> impl Iterator<Item = PackageInfo<'_>> {
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
    ) -> Option<PackageInfo<'_>> {
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

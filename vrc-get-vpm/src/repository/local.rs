use crate::repository::{RemotePackages, RemoteRepository};
use crate::{PackageCollection, PackageInfo, PackageManifest, VersionSelector};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalCachedRepository {
    repo: RemoteRepository,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    headers: IndexMap<Box<str>, Box<str>>,
    #[serde(rename = "vrc-get")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) vrc_get: Option<VrcGetMeta>,
}

impl LocalCachedRepository {
    pub fn new(repo: RemoteRepository, headers: IndexMap<Box<str>, Box<str>>) -> Self {
        Self {
            repo,
            headers,
            vrc_get: None,
        }
    }

    pub fn headers(&self) -> &IndexMap<Box<str>, Box<str>> {
        &self.headers
    }

    pub fn repo(&self) -> &RemoteRepository {
        &self.repo
    }

    pub(crate) fn set_repo(&mut self, mut repo: RemoteRepository) {
        if let Some(id) = self.id() {
            repo.set_id_if_none(|| id.into());
        }
        if let Some(url) = self.url() {
            repo.set_url_if_none(|| url.to_owned());
        }
        self.repo = repo;
    }

    pub(crate) fn set_etag(&mut self, etag: Option<Box<str>>) {
        if let Some(etag) = etag {
            self.vrc_get.get_or_insert_with(Default::default).etag = etag;
        } else if let Some(x) = self.vrc_get.as_mut() {
            x.etag = "".into();
        }
    }

    pub fn url(&self) -> Option<&Url> {
        self.repo().url()
    }

    pub fn id(&self) -> Option<&str> {
        self.repo().id()
    }

    pub fn name(&self) -> Option<&str> {
        self.repo().name()
    }

    pub fn get_versions_of(&self, package: &str) -> impl Iterator<Item = &'_ PackageManifest> {
        self.repo().get_versions_of(package)
    }

    pub fn get_packages(&self) -> impl Iterator<Item = &'_ RemotePackages> {
        self.repo().get_packages()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct VrcGetMeta {
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub etag: Box<str>,
}

impl PackageCollection for LocalCachedRepository {
    fn get_all_packages(&self) -> impl Iterator<Item = PackageInfo> {
        self.repo()
            .get_packages()
            .flat_map(|x| x.all_versions())
            .map(|pkg| PackageInfo::remote(pkg, self))
    }

    fn find_packages(&self, package: &str) -> impl Iterator<Item = PackageInfo> {
        self.get_versions_of(package)
            .map(|pkg| PackageInfo::remote(pkg, self))
    }

    fn find_package_by_name(
        &self,
        package: &str,
        package_selector: VersionSelector,
    ) -> Option<PackageInfo> {
        if let Some(version) = package_selector.as_specific() {
            self.repo
                .get_package_version(package, version)
                .map(|pkg| PackageInfo::remote(pkg, self))
        } else {
            self.find_packages(package)
                .filter(|x| package_selector.satisfies(x.package_json()))
                .max_by_key(|x| x.version())
        }
    }
}

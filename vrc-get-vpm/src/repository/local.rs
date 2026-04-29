use crate::PackageManifest;
use crate::repository::{RemotePackages, RemoteRepository};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalCachedRepository {
    pub(crate) repo: RemoteRepository,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub(crate) headers: IndexMap<Box<str>, Box<str>>,
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

    pub fn set_url(&mut self, url: Url) {
        self.repo.set_url(url);
    }

    pub fn id(&self) -> Option<&str> {
        self.repo().id()
    }

    pub fn name(&self) -> Option<&str> {
        self.repo().name()
    }

    pub fn get_versions_of(
        &self,
        package: &str,
    ) -> impl Iterator<Item = &'_ PackageManifest> + use<'_> {
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

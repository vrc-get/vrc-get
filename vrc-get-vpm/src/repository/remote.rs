use crate::PackageManifest;
use crate::traits::HttpClient;
use crate::utils::json::{JsonError, JsonObject, JsonValue, parse_json_file};
use crate::version::Version;
use crate::{VersionSelector, io};
use futures::prelude::*;
use indexmap::IndexMap;
use itertools::Itertools;
use std::collections::HashMap;
use std::pin::pin;
use std::str::FromStr;
use url::Url;

#[derive(Debug, Clone)]
pub struct RemoteRepository {
    actual: JsonObject,
    name: Option<Box<str>>,
    url: Option<Url>,
    id: Option<Box<str>>,
    packages: IndexMap<Box<str>, RemotePackages>,
}

impl RemoteRepository {
    pub(crate) fn from_json_value(cache: JsonValue) -> Result<Self, JsonError> {
        let actual = cache.into_object()?;
        let actual = actual.clone();
        let name = (actual.get_opt("name"))
            .try_map(JsonValue::into_string)?
            .map(Into::into);
        let url = actual.get_opt("name").parse_opt(|s| s.parse())?;
        let id = (actual.get_opt("id"))
            .try_map(JsonValue::into_string)?
            .map(Into::into);
        let packages = (actual.get_opt("packages"))
            .try_map(JsonValue::into_object)?
            .unwrap_or_default()
            .into_iter()
            .map(|(k, v)| {
                let parsed = RemotePackages::from_json_value(&k, v)?;
                Ok((k.into(), parsed))
            })
            .collect::<Result<IndexMap<_, _>, JsonError>>()?;

        Ok(Self {
            actual,
            name,
            url,
            id,
            packages,
        })
    }

    pub(crate) fn to_json_value(&self) -> JsonValue {
        self.actual.clone().into()
    }

    pub async fn download(
        client: &impl HttpClient,
        url: &Url,
        headers: &IndexMap<Box<str>, Box<str>>,
    ) -> io::Result<(RemoteRepository, Option<Box<str>>)> {
        match Self::download_with_etag(client, url, headers, None).await {
            Ok(None) => unreachable!("downloading without etag should must return Ok(Some)"),
            Ok(Some(repo_and_etag)) => Ok(repo_and_etag),
            Err(err) => Err(err),
        }
    }

    pub async fn download_with_etag(
        client: &impl HttpClient,
        url: &Url,
        headers: &IndexMap<Box<str>, Box<str>>,
        current_etag: Option<&str>,
    ) -> io::Result<Option<(RemoteRepository, Option<Box<str>>)>> {
        let Some((stream, etag)) = client.get_with_etag(url, headers, current_etag).await? else {
            return Ok(None);
        };

        let mut bytes = Vec::new();
        pin!(stream).read_to_end(&mut bytes).await?;

        let mut repo = parse_json_file(&bytes, url, RemoteRepository::from_json_value)?;

        repo.set_url_if_none(|| url.clone());
        Ok(Some((repo, etag)))
    }

    pub(crate) fn set_id_if_none(&mut self, f: impl FnOnce() -> Box<str>) {
        if self.id.is_none() {
            let id = f();
            self.id = Some(id.clone());
            self.actual.insert("id", &*id);
        }
    }

    pub(crate) fn set_url_if_none(&mut self, f: impl FnOnce() -> Url) {
        if self.url.is_none() {
            let url = f();
            self.url = Some(url.clone());
            self.actual.insert("url", url.to_string());
        }
        if self.id.is_none() {
            let url = self.url.as_ref().unwrap().as_str().into();
            self.set_id_if_none(move || url);
        }
    }

    pub fn url(&self) -> Option<&Url> {
        self.url.as_ref()
    }

    pub(crate) fn set_url(&mut self, url: Url) {
        self.url = Some(url);
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn get_versions_of(
        &self,
        package: &str,
    ) -> impl Iterator<Item = &'_ PackageManifest> + use<'_> {
        self.packages
            .get(package)
            .map(RemotePackages::all_versions)
            .into_iter()
            .flatten()
    }

    pub fn get_package(&self, package: &str) -> Option<&RemotePackages> {
        self.packages.get(package)
    }

    pub fn get_packages(&self) -> impl Iterator<Item = &'_ RemotePackages> {
        self.packages.values()
    }

    pub fn get_package_version(&self, name: &str, version: &Version) -> Option<&PackageManifest> {
        self.packages.get(name)?.versions.get(version)
    }
}

#[derive(Debug, Clone)]
pub struct RemotePackages {
    versions: HashMap<Version, PackageManifest>,
}

impl RemotePackages {
    pub fn all_versions(&self) -> impl Iterator<Item = &PackageManifest> {
        self.versions.values()
    }

    pub fn get_latest_may_yanked(&self, selector: VersionSelector) -> Option<&PackageManifest> {
        self.get_latest(selector).or_else(|| {
            self.versions
                .values()
                .filter(|json| selector.satisfies(json))
                .max_by_key(|json| json.version())
        })
    }

    pub fn get_latest(&self, selector: VersionSelector) -> Option<&PackageManifest> {
        if let Some(version) = selector.as_specific() {
            return self.versions.get(version);
        }

        self.versions
            .values()
            .filter(|json| selector.satisfies(json))
            .filter(|json| !json.is_yanked())
            .max_by_key(|json| json.version())
    }

    pub fn get_version(&self, version: &Version) -> Option<&PackageManifest> {
        self.versions.get(version)
    }
}

impl RemotePackages {
    fn from_json_value(name: &str, value: JsonValue) -> Result<RemotePackages, JsonError> {
        let object = value.into_object()?;
        let versions = object
            .get_opt("versions")
            .try_map(|value| value.into_object())?
            .unwrap_or_default();
        let versions = versions
            .into_keys_parsed(|s| Version::from_str(&s))
            .map_ok(
                |(version, value)| match PackageManifest::from_json_value(value) {
                    Ok(manifest) => Some((version, manifest)),
                    Err(err) => {
                        log::warn!(
                            "Error deserializing package manifest for {name}@{version}: {err}",
                        );
                        None
                    }
                },
            )
            .flatten_ok()
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(RemotePackages { versions })
    }
}

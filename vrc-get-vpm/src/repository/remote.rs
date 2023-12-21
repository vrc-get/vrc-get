use crate::structs::package::PackageJson;
use crate::traits::HttpClient;
use crate::version::Version;
use futures::prelude::*;
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io;
use std::pin::pin;
use url::Url;

type JsonMap = Map<String, Value>;

#[derive(Debug, Clone)]
pub struct RemoteRepository {
    actual: JsonMap,
    parsed: ParsedRepository,
}

#[derive(Deserialize, Debug, Clone)]
struct ParsedRepository {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    url: Option<Url>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    packages: HashMap<String, RemotePackages>,
}

impl RemoteRepository {
    pub fn parse(cache: JsonMap) -> serde_json::Result<Self> {
        Ok(Self {
            parsed: serde_json::from_value(Value::Object(cache.clone()))?,
            actual: cache,
        })
    }

    pub async fn download(
        client: &impl HttpClient,
        url: &Url,
        headers: &IndexMap<String, String>,
    ) -> io::Result<RemoteRepository> {
        match Self::download_with_etag(client, url, headers, None).await {
            Ok(None) => unreachable!("downloading without etag should must return Ok(Some)"),
            Ok(Some((repo, _))) => Ok(repo),
            Err(err) => Err(err),
        }
    }

    pub async fn download_with_etag(
        client: &impl HttpClient,
        url: &Url,
        headers: &IndexMap<String, String>,
        current_etag: Option<&str>,
    ) -> io::Result<Option<(RemoteRepository, Option<String>)>> {
        let Some((mut stream, etag)) = client.get_with_etag(url, headers, current_etag).await?
        else {
            return Ok(None);
        };

        let mut bytes = Vec::new();
        pin!(stream).read_to_end(&mut bytes).await?;

        let no_bom = bytes
            .strip_prefix(b"\xEF\xBB\xBF")
            .unwrap_or(bytes.as_ref());
        let json = serde_json::from_slice(no_bom)?;

        let mut repo = RemoteRepository::parse(json)?;
        repo.set_url_if_none(|| url.clone());
        Ok(Some((repo, etag)))
    }

    pub fn set_id_if_none(&mut self, f: impl FnOnce() -> String) {
        if self.parsed.id.is_none() {
            let id = f();
            self.parsed.id = Some(id.clone());
            self.actual.insert("id".to_owned(), Value::String(id));
        }
    }

    pub fn set_url_if_none(&mut self, f: impl FnOnce() -> Url) {
        if self.parsed.url.is_none() {
            let url = f();
            self.parsed.url = Some(url.clone());
            self.actual
                .insert("url".to_owned(), Value::String(url.to_string()));
            if self.parsed.id.is_none() {
                let url = self.parsed.url.as_ref().unwrap().to_string();
                self.set_id_if_none(move || url);
            }
        }
    }

    pub fn url(&self) -> Option<&Url> {
        self.parsed.url.as_ref()
    }

    pub fn id(&self) -> Option<&str> {
        self.parsed.id.as_deref()
    }

    pub fn name(&self) -> Option<&str> {
        self.parsed.name.as_deref()
    }

    pub fn get_versions_of(&self, package: &str) -> impl Iterator<Item = &'_ PackageJson> {
        self.parsed
            .packages
            .get(package)
            .map(RemotePackages::all_versions)
            .into_iter()
            .flatten()
    }

    pub fn get_packages(&self) -> impl Iterator<Item = &'_ RemotePackages> {
        self.parsed.packages.values()
    }

    pub fn get_package_version(&self, name: &str, version: &Version) -> Option<&PackageJson> {
        self.parsed
            .packages
            .get(name)?
            .versions
            .get(&version.to_string())
    }
}

impl Serialize for RemoteRepository {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.actual.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RemoteRepository {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        let map = JsonMap::deserialize(deserializer)?;
        Self::parse(map).map_err(Error::custom)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct RemotePackages {
    #[serde(default)]
    versions: HashMap<String, PackageJson>,
}

impl RemotePackages {
    pub fn all_versions(&self) -> impl Iterator<Item = &PackageJson> {
        self.versions.values()
    }
}

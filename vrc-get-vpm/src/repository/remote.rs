use crate::PackageManifest;
use crate::traits::HttpClient;
use crate::utils::{deserialize_json, deserialize_json_slice};
use crate::version::Version;
use crate::{VersionSelector, io};
use futures::prelude::*;
use indexmap::IndexMap;
use serde::de::{DeserializeSeed, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};
use std::collections::HashMap;
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
    name: Option<Box<str>>,
    #[serde(default)]
    url: Option<Url>,
    #[serde(default)]
    id: Option<Box<str>>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_packages")]
    packages: IndexMap<Box<str>, RemotePackages>,
}

impl RemoteRepository {
    pub fn parse(cache: JsonMap) -> io::Result<Self> {
        Ok(Self {
            parsed: deserialize_json(Value::Object(cache.clone()))?,
            actual: cache,
        })
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

        let no_bom = bytes
            .strip_prefix(b"\xEF\xBB\xBF")
            .unwrap_or(bytes.as_ref());
        let json = deserialize_json_slice(no_bom)?;

        let mut repo = RemoteRepository::parse(json)?;
        repo.set_url_if_none(|| url.clone());
        Ok(Some((repo, etag)))
    }

    pub(crate) fn set_id_if_none(&mut self, f: impl FnOnce() -> Box<str>) {
        if self.parsed.id.is_none() {
            let id = f();
            self.parsed.id = Some(id.clone());
            self.actual
                .insert("id".to_owned(), Value::String(id.into()));
        }
    }

    pub(crate) fn set_url_if_none(&mut self, f: impl FnOnce() -> Url) {
        if self.parsed.url.is_none() {
            let url = f();
            self.parsed.url = Some(url.clone());
            self.actual
                .insert("url".to_owned(), Value::String(url.to_string()));
        }
        if self.parsed.id.is_none() {
            let url = self.parsed.url.as_ref().unwrap().as_str().into();
            self.set_id_if_none(move || url);
        }
    }

    pub fn url(&self) -> Option<&Url> {
        self.parsed.url.as_ref()
    }

    pub(crate) fn set_url(&mut self, url: Url) {
        self.parsed.url = Some(url);
    }

    pub fn id(&self) -> Option<&str> {
        self.parsed.id.as_deref()
    }

    pub fn name(&self) -> Option<&str> {
        self.parsed.name.as_deref()
    }

    pub fn get_versions_of(
        &self,
        package: &str,
    ) -> impl Iterator<Item = &'_ PackageManifest> + use<'_> {
        self.parsed
            .packages
            .get(package)
            .map(RemotePackages::all_versions)
            .into_iter()
            .flatten()
    }

    pub fn get_package(&self, package: &str) -> Option<&RemotePackages> {
        self.parsed.packages.get(package)
    }

    pub fn get_packages(&self) -> impl Iterator<Item = &'_ RemotePackages> {
        self.parsed.packages.values()
    }

    pub fn get_package_version(&self, name: &str, version: &Version) -> Option<&PackageManifest> {
        self.parsed.packages.get(name)?.versions.get(version)
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

fn deserialize_packages<'de, D>(
    deserializer: D,
) -> Result<IndexMap<Box<str>, RemotePackages>, D::Error>
where
    D: Deserializer<'de>,
{
    struct VisitorImpl;

    impl<'de> Visitor<'de> for VisitorImpl {
        type Value = IndexMap<Box<str>, RemotePackages>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a map of package names to package versions")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut packages = IndexMap::new();
            while let Some(name) = map.next_key::<Box<str>>()? {
                let versions = map.next_value_seed(PackageNameToRemotePackages(&name))?;
                packages.insert(name, versions);
            }
            Ok(packages)
        }
    }

    deserializer.deserialize_map(VisitorImpl)
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
            .clone()
            .filter(|json| !json.is_yanked())
            .max_by_key(|json| json.version())
    }

    pub fn get_version(&self, version: &Version) -> Option<&PackageManifest> {
        self.versions.get(version)
    }
}

struct PackageNameToRemotePackages<'a>(&'a str);

impl<'de> DeserializeSeed<'de> for PackageNameToRemotePackages<'_> {
    type Value = RemotePackages;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VisitorImpl<'a>(&'a str);

        impl<'de> Visitor<'de> for VisitorImpl<'_> {
            type Value = RemotePackages;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map of package versions")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut versions = HashMap::new();
                while let Some(key) = map.next_key::<&'de str>()? {
                    if key == "versions" {
                        versions = map.next_value_seed(PackageNameToVersions(self.0))?;
                    }
                }
                Ok(RemotePackages { versions })
            }
        }

        deserializer.deserialize_struct("RemotePackages", &["versions"], VisitorImpl(self.0))
    }
}

struct PackageNameToVersions<'a>(&'a str);

impl<'de> DeserializeSeed<'de> for PackageNameToVersions<'_> {
    type Value = HashMap<Version, PackageManifest>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VisitorImpl<'a>(&'a str);

        impl<'de> Visitor<'de> for VisitorImpl<'_> {
            type Value = HashMap<Version, PackageManifest>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map of versions to package manifests")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut versions = HashMap::new();
                while let Some(version) = map.next_key::<Version>()? {
                    let manifest = map.next_value_seed(ErrorProofManifest(self.0, &version))?;
                    if let Some(manifest) = manifest {
                        versions.insert(version, manifest);
                    }
                }
                Ok(versions)
            }
        }

        deserializer.deserialize_map(VisitorImpl(self.0))
    }
}

struct ErrorProofManifest<'a>(&'a str, &'a Version);

impl<'de> DeserializeSeed<'de> for ErrorProofManifest<'_> {
    type Value = Option<PackageManifest>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_value::Value::deserialize(deserializer)?;
        match PackageManifest::deserialize(value) {
            Ok(manifest) => Ok(Some(manifest)),
            Err(err) => {
                log::warn!(
                    "Error deserializing package manifest for {}@{}: {err}",
                    self.0,
                    self.1,
                );
                Ok(None)
            }
        }
    }
}

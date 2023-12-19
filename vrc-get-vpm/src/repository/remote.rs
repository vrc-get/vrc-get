use std::collections::HashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};
use crate::structs::package;

type JsonMap = Map<String, Value>;

#[derive(Debug, Clone)]
pub struct RemoteRepository {
    actual: JsonMap,
    parsed: ParsedRepository,
}

impl RemoteRepository {
    pub fn parse(cache: JsonMap) -> serde_json::Result<Self> {
        Ok(Self {
            parsed: serde_json::from_value(Value::Object(cache.clone()))?,
            actual: cache,
        })
    }

    pub fn set_id_if_none(&mut self, f: impl FnOnce() -> String) {
        if let None = self.parsed.id {
            let id = f();
            self.parsed.id = Some(id.clone());
            self.actual.insert("id".to_owned(), Value::String(id));
        }
    }

    pub fn set_url_if_none(&mut self, f: impl FnOnce() -> String) {
        if let None = self.parsed.url {
            let url = f();
            self.parsed.url = Some(url.clone());
            self.actual.insert("url".to_owned(), Value::String(url));
            if let None = self.parsed.id {
                let url = self.parsed.url.clone().unwrap();
                self.set_id_if_none(move || url);
            }
        }
    }

    pub fn url(&self) -> Option<&str> {
        self.parsed.url.as_deref()
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
    ) -> impl Iterator<Item = &'_ package::PackageJson> {
        self.parsed
            .packages
            .get(package)
            .map(|x| x.versions.values())
            .into_iter()
            .flatten()
    }

    pub fn get_packages(&self) -> impl Iterator<Item = &'_ PackageVersions> {
        self.parsed.packages.values().into_iter()
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
struct ParsedRepository {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    packages: HashMap<String, PackageVersions>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PackageVersions {
    #[serde(default)]
    pub versions: HashMap<String, package::PackageJson>,
}

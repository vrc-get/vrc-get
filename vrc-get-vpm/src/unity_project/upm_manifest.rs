use crate::io;
use crate::io::DefaultProjectIo;
use crate::utils::{JsonMapExt, SaveController, load_json_or_default, save_json};
use crate::version::Version;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fmt::Formatter;
use std::str::FromStr;

const MANIFEST_PATH: &str = "Packages/manifest.json";

#[derive(Debug, Default, Deserialize)]
struct Parsed {
    #[serde(default)]
    dependencies: HashMap<Box<str>, UpmDependency>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub(super) enum UpmDependency {
    // minimum version name. build meta is not supported by upm
    Version(Version),
    // Other Notation including local file and git url
    OtherNotation(Box<str>),
}

impl<'de> Deserialize<'de> for UpmDependency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = UpmDependency;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("one of: a 'SemVer' compatible value; a value starting with 'file:'; a Git URL starting with 'git:' or 'git+', or ending with '.git'.")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if let Ok(semver) = Version::from_str(v) {
                    Ok(UpmDependency::Version(semver))
                } else {
                    Ok(UpmDependency::OtherNotation(v.into()))
                }
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if let Ok(semver) = Version::from_str(&v) {
                    Ok(UpmDependency::Version(semver))
                } else {
                    Ok(UpmDependency::OtherNotation(v.into_boxed_str()))
                }
            }
        }

        deserializer.deserialize_string(Visitor)
    }
}

#[derive(Default, Debug)]
struct AsJson {
    as_json: Parsed,
    raw: Map<String, Value>,
}

impl<'de> Deserialize<'de> for AsJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw: Map<String, Value> = Map::<String, Value>::deserialize(deserializer)?;
        let raw_value = Value::Object(raw);
        let as_json = Parsed::deserialize(&raw_value).map_err(Error::custom)?;
        let raw = match raw_value {
            Value::Object(map) => map,
            _ => unreachable!(),
        };
        Ok(Self { as_json, raw })
    }
}

impl Serialize for AsJson {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.raw.serialize(serializer)
    }
}

#[derive(Debug)]
pub(super) struct UpmManifest {
    controller: SaveController<AsJson>,
}

impl UpmManifest {
    pub(super) async fn load(io: &DefaultProjectIo) -> io::Result<Self> {
        Ok(Self {
            controller: SaveController::new(
                load_json_or_default(io, MANIFEST_PATH.as_ref()).await?,
            ),
        })
    }

    #[allow(dead_code)]
    pub(super) fn dependencies(&self) -> impl Iterator<Item = (&str, &UpmDependency)> {
        self.controller
            .as_json
            .dependencies
            .iter()
            .map(|(name, dep)| (name.as_ref(), dep))
    }

    #[allow(dead_code)]
    pub(super) fn get_dependency(&self, package: &str) -> Option<&UpmDependency> {
        self.controller.as_json.dependencies.get(package)
    }

    #[allow(dead_code)]
    pub(super) fn add_dependency(&mut self, name: &str, version: Version) {
        self.controller
            .as_mut()
            .raw
            .get_or_put_mut("dependencies", Map::new)
            .as_object_mut()
            .unwrap()
            .insert(name.to_string(), Value::String(version.to_string()));
        self.controller
            .as_mut()
            .as_json
            .dependencies
            .insert(name.into(), UpmDependency::Version(version));
    }

    pub(super) fn remove_dependency(&mut self, name: &str) {
        self.controller
            .as_mut()
            .raw
            .get_mut("dependencies")
            .and_then(|x| x.as_object_mut())
            .map(|x| x.remove(name));
        self.controller.as_mut().as_json.dependencies.remove(name);
    }

    pub(super) async fn save(&mut self, io: &DefaultProjectIo) -> io::Result<()> {
        self.controller
            .save(|json| save_json(io, MANIFEST_PATH.as_ref(), json))
            .await
    }
}

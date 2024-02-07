use crate::io;
use crate::io::ProjectIo;
use crate::utils::{load_json_or_default, to_vec_pretty_os_eol, JsonMapExt};
use crate::version::Version;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fmt::Formatter;
use std::str::FromStr;

const MANIFEST_PATH: &str = "Packages/manifest.json";

#[derive(Debug, Deserialize)]
struct Parsed {
    #[serde(default)]
    dependencies: HashMap<Box<str>, UpmDependency>,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub(super) struct UpmManifest {
    as_json: Parsed,
    raw: Map<String, Value>,
    changed: bool,
}

impl UpmManifest {
    pub(super) async fn load(io: &impl ProjectIo) -> io::Result<Self> {
        let raw: Map<String, Value> = load_json_or_default(io, MANIFEST_PATH.as_ref()).await?;
        let raw_value = Value::Object(raw);
        let as_json = Parsed::deserialize(&raw_value)?;
        let raw = match raw_value {
            Value::Object(map) => map,
            _ => unreachable!(),
        };
        Ok(Self {
            as_json,
            raw,
            changed: false,
        })
    }

    #[allow(dead_code)]
    pub(super) fn dependencies(&self) -> impl Iterator<Item = (&str, &UpmDependency)> {
        self.as_json
            .dependencies
            .iter()
            .map(|(name, dep)| (name.as_ref(), dep))
    }

    #[allow(dead_code)]
    pub(super) fn get_dependency(&self, package: &str) -> Option<&UpmDependency> {
        self.as_json.dependencies.get(package)
    }

    #[allow(dead_code)]
    pub(super) fn add_dependency(&mut self, name: &str, version: Version) {
        self.raw
            .get_or_put_mut("dependencies", Map::new)
            .as_object_mut()
            .unwrap()
            .insert(name.to_string(), Value::String(version.to_string()));
        self.as_json
            .dependencies
            .insert(name.into(), UpmDependency::Version(version));
        self.changed = true;
    }

    pub(super) fn remove_dependency(&mut self, name: &str) {
        self.raw
            .get_mut("dependencies")
            .and_then(|x| x.as_object_mut())
            .map(|x| x.remove(name));
        self.as_json.dependencies.remove(name);
        self.changed = true;
    }

    pub(super) async fn save(&mut self, io: &impl ProjectIo) -> io::Result<()> {
        if self.changed {
            let json = to_vec_pretty_os_eol(&self.raw)?;
            io.write(MANIFEST_PATH.as_ref(), json.as_ref()).await?;
            self.changed = false;
        }
        Ok(())
    }
}

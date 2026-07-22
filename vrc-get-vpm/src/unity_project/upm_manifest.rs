use crate::io;
use crate::io::DefaultProjectIo;
use crate::utils::SaveController;
use crate::utils::json::{JsonError, JsonObject, JsonValue, save_json, try_load_json};
use crate::version::Version;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

const MANIFEST_PATH: &str = "Packages/manifest.json";

#[derive(Debug, Default)]
struct Parsed {
    dependencies: HashMap<Box<str>, UpmDependency>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub(super) enum UpmDependency {
    Version(Version),
    OtherNotation(Box<str>),
}

impl UpmDependency {
    fn from_json_value(value: JsonValue) -> Result<Self, JsonError> {
        let v = value.into_string()?;
        if let Ok(semver) = Version::from_str(&v) {
            Ok(UpmDependency::Version(semver))
        } else {
            Ok(UpmDependency::OtherNotation(v.into()))
        }
    }
}

#[derive(Default, Debug)]
struct AsJson {
    as_json: Parsed,
    raw: JsonObject,
}

impl AsJson {
    fn from_json_value(value: JsonValue) -> Result<Self, JsonError> {
        let raw = value.into_object()?;
        Ok(Self {
            as_json: Parsed::from_json_value(raw.clone().into())?,
            raw,
        })
    }
}

impl Parsed {
    fn from_json_value(value: JsonValue) -> Result<Self, JsonError> {
        let raw = value.into_object()?;
        let dependencies = (raw.get_opt("dependencies"))
            .try_map(|value| {
                (value.into_object())?
                    .into_iter()
                    .map(|(k, v)| Ok((k.into(), UpmDependency::from_json_value(v)?)))
                    .collect::<Result<_, JsonError>>()
            })?
            .unwrap_or(HashMap::new());
        Ok(Self { dependencies })
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
                try_load_json(io, MANIFEST_PATH.as_ref(), AsJson::from_json_value)
                    .await?
                    .unwrap_or(AsJson::default()),
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
            .get_or_insert_mut("dependencies", JsonObject::new().into())
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
            .save(|json| {
                save_json(
                    io,
                    MANIFEST_PATH.as_ref(),
                    JsonValue::from(json.raw.clone()),
                )
            })
            .await
    }
}

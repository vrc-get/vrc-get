use crate::io;
use crate::io::DefaultProjectIo;
use crate::unity_project::LockedDependencyInfo;
use crate::utils::SaveController;
use crate::utils::json::{
    JsonError, JsonObject, JsonValue, save_json, to_vec_pretty_os_eol, try_load_json,
};
use crate::version::{DependencyRange, Version, VersionRange};
use indexmap::IndexMap;
use std::str::FromStr;

const MANIFEST_PATH: &str = "Packages/vpm-manifest.json";

#[derive(Debug, Default)]
struct AsJson {
    dependencies: IndexMap<Box<str>, VpmDependency>,
    locked: IndexMap<Box<str>, VpmLockedDependency>,
}

#[derive(Debug, Clone)]
struct VpmDependency {
    pub version: DependencyRange,
}

#[derive(Debug, Clone)]
struct VpmLockedDependency {
    pub version: Version,
    pub dependencies: Option<IndexMap<Box<str>, VersionRange>>,
}

impl AsJson {
    fn from_json_value(value: JsonValue) -> Result<Self, JsonError> {
        let object = value.into_object()?;
        Ok(Self {
            dependencies: object
                .get_opt("dependencies")
                .try_map(|value| {
                    (value.into_object())?
                        .into_iter()
                        .map(|(key, value)| {
                            Ok((key.into(), VpmDependency::from_json_value(value)?))
                        })
                        .collect::<Result<_, JsonError>>()
                })?
                .unwrap_or_else(IndexMap::new),

            locked: object
                .get_opt("locked")
                .try_map(|value| {
                    (value.into_object())?
                        .into_iter()
                        .map(|(key, value)| {
                            Ok((key.into(), VpmLockedDependency::from_json_value(value)?))
                        })
                        .collect::<Result<_, JsonError>>()
                })?
                .unwrap_or_else(IndexMap::new),
        })
    }

    fn to_json_value(&self) -> JsonValue {
        let mut object = JsonObject::new();

        object.insert(
            "dependencies",
            self.dependencies
                .iter()
                .map(|(name, dep)| (name.to_string(), dep.to_json_value()))
                .collect::<JsonObject>(),
        );
        object.insert(
            "locked",
            self.locked
                .iter()
                .map(|(name, dep)| (name.to_string(), dep.to_json_value()))
                .collect::<JsonObject>(),
        );
        object.into()
    }
}

impl VpmDependency {
    fn from_json_value(value: JsonValue) -> Result<Self, JsonError> {
        let mut object = value.into_object()?;
        Ok(Self {
            version: object
                .get_req("version")?
                .parse_req(|s| DependencyRange::from_str(&s))?,
        })
    }

    fn to_json_value(&self) -> JsonValue {
        let mut object = JsonObject::new();
        object.insert("version", self.version.to_string());
        object.into()
    }
}

impl VpmLockedDependency {
    fn from_json_value(value: JsonValue) -> Result<Self, JsonError> {
        let mut object = value.into_object()?;
        Ok(Self {
            version: object
                .get_req("version")?
                .parse_req(|s| Version::from_str(&s))?,

            dependencies: object.get_opt("dependencies").try_map(|value| {
                (value.into_object())?
                    .into_iter()
                    .map(|(key, value)| {
                        Ok((key.into(), value.parse_req(|s| VersionRange::from_str(&s))?))
                    })
                    .collect::<Result<_, JsonError>>()
            })?,
        })
    }

    fn to_json_value(&self) -> JsonValue {
        let mut object = JsonObject::new();
        object.insert("version", self.version.to_string());
        if let Some(dependencies) = &self.dependencies {
            object.insert(
                "dependencies",
                dependencies
                    .iter()
                    .map(|(name, value)| (name.as_ref(), value.to_string()))
                    .collect::<JsonObject>(),
            );
        }
        object.into()
    }
}

#[derive(Debug)]
pub(super) struct VpmManifest {
    controller: SaveController<AsJson>,
}

impl VpmManifest {
    pub(super) async fn load(io: &DefaultProjectIo) -> io::Result<Self> {
        Ok(Self {
            controller: SaveController::new(
                try_load_json(io, MANIFEST_PATH.as_ref(), AsJson::from_json_value)
                    .await?
                    .unwrap_or_else(Default::default),
            ),
        })
    }

    pub(super) fn dependencies(&self) -> impl Iterator<Item = (&str, &DependencyRange)> {
        self.controller
            .dependencies
            .iter()
            .map(|(name, dep)| (name.as_ref(), &dep.version))
    }

    pub(super) fn get_dependency(&self, package: &str) -> Option<&DependencyRange> {
        self.controller
            .dependencies
            .get(package)
            .map(|x| &x.version)
    }

    pub(super) fn all_locked(&self) -> impl Iterator<Item = LockedDependencyInfo<'_>> {
        self.controller.locked.iter().map(|(name, dep)| {
            LockedDependencyInfo::new(name.as_ref(), &dep.version, dep.dependencies.as_ref())
        })
    }

    pub(super) fn get_locked(&self, package: &str) -> Option<LockedDependencyInfo<'_>> {
        self.controller
            .locked
            .get_key_value(package)
            .map(|(package, x)| {
                LockedDependencyInfo::new(package, &x.version, x.dependencies.as_ref())
            })
    }

    pub(super) fn add_dependency(&mut self, name: &str, version: DependencyRange) {
        self.controller
            .as_mut()
            .dependencies
            .insert(name.into(), VpmDependency { version });
    }

    pub(super) fn add_locked(
        &mut self,
        name: &str,
        version: Version,
        dependencies: IndexMap<Box<str>, VersionRange>,
    ) {
        self.controller.as_mut().locked.insert(
            name.into(),
            VpmLockedDependency {
                version,
                dependencies: Some(dependencies),
            },
        );
    }

    pub(crate) fn remove_packages<'a>(&mut self, names: impl Iterator<Item = &'a str>) {
        for name in names {
            self.controller.as_mut().locked.shift_remove(name);
            self.controller.as_mut().dependencies.shift_remove(name);
        }
    }

    pub(crate) fn has_any(&self) -> bool {
        !self.controller.locked.is_empty() || !self.controller.dependencies.is_empty()
    }

    pub(super) async fn save(&mut self, io: &DefaultProjectIo) -> io::Result<()> {
        self.controller
            .save(|json| save_json(io, MANIFEST_PATH.as_ref(), json.to_json_value()))
            .await
    }

    pub(super) fn to_json(&self) -> io::Result<Vec<u8>> {
        to_vec_pretty_os_eol(&self.controller.to_json_value())
    }
}

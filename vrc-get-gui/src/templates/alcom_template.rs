#![doc = include_str!("./alcom_template.md")]

use indexmap::IndexMap;
use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;
use vrc_get_vpm::version::VersionRange;

static MAGIC: &str = "com.anatawa12.vrc-get.custom-template";

#[derive(Serialize, Deserialize)]
struct MagicParser {
    #[serde(rename = "$type")]
    ty: String,
    #[serde(rename = "formatVersion")]
    format_version: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AlcomTemplateContent {
    pub display_name: String,
    pub update_date: Option<chrono::DateTime<chrono::offset::Utc>>,
    pub id: Option<TemplateId>,
    pub base: TemplateId,
    pub unity_version: Option<VersionRange>,
    #[serde(default)]
    pub vpm_dependencies: IndexMap<String, VersionRange>,
    #[serde(default)]
    pub unity_packages: Vec<PathBuf>,
}

struct TemplateId(String);

impl<'de> Deserialize<'de> for TemplateId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let id = String::deserialize(deserializer)?;
        if id.is_empty()
            || id
                .chars()
                .any(|c| !matches!(c, '0'..='9' | 'A'..='Z' | 'a'..='z' | '.' | '_' | '-'))
        {
            return Err(D::Error::invalid_value(
                Unexpected::Str(&id),
                &"a valid alcom template id",
            ));
        }
        Ok(Self(id))
    }
}

impl Serialize for TemplateId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        String::serialize(&self.0, serializer)
    }
}

#[derive(Serialize)]
struct AlcomTemplateSerialize {
    #[serde(flatten)]
    magic: MagicParser,
    #[serde(flatten)]
    content: AlcomTemplateContent,
}

#[derive(Clone)]
pub struct AlcomTemplate {
    pub display_name: String,
    pub update_date: Option<chrono::DateTime<chrono::offset::Utc>>,
    pub id: Option<String>,
    pub base: String,
    pub unity_version: Option<VersionRange>,
    pub vpm_dependencies: IndexMap<String, VersionRange>,
    pub unity_packages: Vec<PathBuf>,
}

pub fn parse_alcom_template(json: &[u8]) -> serde_json::Result<AlcomTemplate> {
    // first, parse magic and format version
    let magic: MagicParser = serde_json::from_slice(json)?;
    if magic.ty != MAGIC {
        return Err(serde_json::Error::custom("Invalid $type"));
    }

    let Some((major, _minor)) = parse_format_version(&magic.format_version) else {
        return Err(serde_json::Error::custom(format!(
            "Unsupported formatVersion: {}",
            magic.format_version
        )));
    };

    if major != 1 {
        return Err(serde_json::Error::custom(format!(
            "Unsupported formatVersion: {}",
            magic.format_version
        )));
    }

    // we've checked the version is correct! Parse the contents now.

    let template = serde_json::from_slice::<AlcomTemplateContent>(json)?;

    // few validations
    if let Some(id_wrapper) = &template.id { // id_wrapper is &TemplateId
        // Reserve `com.anatawa12.vrc-get.*` except for known namespaces which are allowed for user-created
        // templates (".user") and imported VCC templates (".vcc").
        if id_wrapper.0.starts_with("com.anatawa12.vrc-get")
            && !(id_wrapper.0.starts_with("com.anatawa12.vrc-get.user.")
                || id_wrapper.0.starts_with("com.anatawa12.vrc-get.vcc."))
        {
            return Err(serde_json::Error::invalid_value(
                Unexpected::Str(&id_wrapper.0),
                &"a valid alcom template id (reserved id, or disallowed com.anatawa12.vrc-get.* namespace)",
            ));
        }
    }

    // Validation for template.base
    if template.base.0.starts_with("com.anatawa12.vrc-get.user.") {
        return Err(serde_json::Error::invalid_value(
            Unexpected::Str(&template.base.0),
            &"user-defined templates (com.anatawa12.vrc-get.user.*) cannot be used as a base template",
        ));
    }

    Ok(AlcomTemplate {
        display_name: template.display_name,
        update_date: template.update_date,
        id: template.id.map(|id_wrapper| id_wrapper.0),
        base: template.base.0,
        unity_version: template.unity_version,
        vpm_dependencies: template.vpm_dependencies,
        unity_packages: template.unity_packages,
    })
}

fn parse_format_version(json: &str) -> Option<(u32, u32)> {
    let (major, minor) = json.split_once('.')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
}

#[allow(dead_code)]
pub fn serialize_alcom_template(template: AlcomTemplate) -> serde_json::Result<Vec<u8>> {
    // TODO: When we extended format, detect and update format_version
    let serialize = AlcomTemplateSerialize {
        magic: MagicParser {
            ty: MAGIC.into(),
            format_version: "1.0".into(),
        },
        content: AlcomTemplateContent {
            display_name: template.display_name,
            update_date: template.update_date,
            id: template.id.clone().map(TemplateId),
            base: TemplateId(template.base.clone()),
            unity_version: template.unity_version,
            vpm_dependencies: template.vpm_dependencies,
            unity_packages: template.unity_packages,
        },
    };
    serde_json::to_vec_pretty(&serialize)
}

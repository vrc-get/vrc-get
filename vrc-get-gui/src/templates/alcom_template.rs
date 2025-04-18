#![doc = include_str!("./alcom_template.md")]

use serde::de::{Error, Unexpected};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use vrc_get_vpm::version::VersionRange;

static MAGIC: &str = "com.anatawa12.vrc-get.custom-template";

#[derive(Deserialize)]
struct MagicParser {
    #[serde(rename = "$type")]
    ty: String,
    #[serde(rename = "formatVersion")]
    format_version: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlcomTemplate {
    pub display_name: String,
    pub update_date: String,
    pub id: Option<String>,
    pub base: String,
    pub unity_version: Option<VersionRange>,
    #[serde(default)]
    pub vpm_dependencies: HashMap<String, VersionRange>,
    #[serde(default)]
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

    let template = serde_json::from_slice::<AlcomTemplate>(json)?;

    // few validations
    if let Some(id) = &template.id {
        // /[a-zA-Z0-9._-]+/
        if id.is_empty()
            || id
                .chars()
                .any(|c| !matches!(c, '0'..='9' | 'A'..='Z' | 'a'..='z' | '.' | '_' | '-'))
        {
            return Err(serde_json::Error::invalid_value(
                Unexpected::Str(id),
                &"a valid alcom template id",
            ));
        }
    }

    Ok(template)
}

fn parse_format_version(json: &str) -> Option<(u32, u32)> {
    let (major, minor) = json.split_once('.')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
}

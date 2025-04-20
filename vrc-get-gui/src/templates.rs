use futures::{AsyncReadExt, TryStreamExt, try_join};
use indexmap::IndexMap;
use log::warn;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io;
use std::path::Path;
use vrc_get_vpm::io::{DefaultEnvironmentIo, DefaultProjectIo, DirEntry, EnvironmentIo, IoTrait};

use crate::commands::UnityProject;
pub use alcom_template::*;
use vrc_get_vpm::version::UnityVersion;

pub mod alcom_template;

include!(concat!(env!("OUT_DIR"), "/templates.rs"));

const AVATARS_TEMPLATE_ID: &str = "com.anatawa12.vrc-get.vrchat.avatars";
const WORLDS_TEMPLATE_ID: &str = "com.anatawa12.vrc-get.vrchat.worlds";
const BLANK_TEMPLATE_ID: &str = "com.anatawa12.vrc-get.blank";
const VRCHAT_UNITY_VERSIONS: &[UnityVersion] = &[
    UnityVersion::new_f1(2019, 4, 31),
    UnityVersion::new_f1(2022, 3, 6),
    UnityVersion::new_f1(2022, 3, 22),
];
const VCC_TEMPLATE_PREFIX: &str = "com.anatawa12.vrc-get.vcc.";
const UNNAMED_TEMPLATE_PREFIX: &str = "com.anatawa12.vrc-get.user.";

#[derive(Serialize, Deserialize)]
pub struct ProjectTemplateInfo {
    pub display_name: String,
    pub id: String,
    pub unity_versions: Vec<UnityVersion>,
    pub alcom_template: Option<AlcomTemplate>,
    // If the base template does not exist, the template is not available.
    pub available: bool,
}

pub async fn load_resolve_all_templates(
    io: &DefaultEnvironmentIo,
    unity_versions: &[UnityVersion],
) -> io::Result<Vec<ProjectTemplateInfo>> {
    let (alcom, vcc) = try_join!(
        load_resolve_alcom_templates(io, unity_versions),
        load_vcc_templates(io)
    )?;
    Ok(alcom.into_iter().chain(vcc.into_iter()).collect())
}

pub async fn load_vcc_templates(io: &DefaultEnvironmentIo) -> io::Result<Vec<ProjectTemplateInfo>> {
    let mut templates = Vec::new();

    let path = io.resolve("Templates".as_ref());
    let mut dir = io.read_dir("Templates".as_ref()).await?;
    while let Some(dir) = dir.try_next().await? {
        if !dir.file_type().await?.is_dir() {
            continue;
        }

        let Ok(name) = dir.file_name().into_string() else {
            continue;
        };

        let path = path.join(&name);

        // check package.json
        let Ok(pkg_json) = tokio::fs::metadata(path.join("package.json")).await else {
            continue;
        };
        if !pkg_json.is_file() {
            continue;
        }

        match UnityProject::load(DefaultProjectIo::new(path.into())).await {
            Err(e) => {
                warn!("failed to load user template {name}: {e}");
            }
            Ok(ref p) if !p.is_valid().await => {
                warn!("failed to load user template {name}: invalid project");
            }
            Ok(p) => templates.push(ProjectTemplateInfo {
                display_name: name.clone(),
                id: format!("{}{}", VCC_TEMPLATE_PREFIX, name),
                unity_versions: vec![p.unity_version().unwrap()],
                alcom_template: None,
                available: true,
            }),
        }
    }

    Ok(templates)
}

pub async fn load_resolve_alcom_templates(
    io: &DefaultEnvironmentIo,
    unity_versions: &[UnityVersion],
) -> io::Result<Vec<ProjectTemplateInfo>> {
    let templates = load_alcom_templates(io).await?;

    let mut template_by_id = IndexMap::<String, ProjectTemplateInfo>::new();

    // builtin templates at first
    template_by_id.insert(
        AVATARS_TEMPLATE_ID.into(),
        ProjectTemplateInfo {
            display_name: "VRChat Avatars".into(),
            id: AVATARS_TEMPLATE_ID.into(),
            unity_versions: VRCHAT_UNITY_VERSIONS.into(),
            alcom_template: None,
            available: true,
        },
    );
    template_by_id.insert(
        WORLDS_TEMPLATE_ID.into(),
        ProjectTemplateInfo {
            display_name: "VRChat Worlds".into(),
            id: WORLDS_TEMPLATE_ID.into(),
            unity_versions: VRCHAT_UNITY_VERSIONS.into(),
            alcom_template: None,
            available: true,
        },
    );
    template_by_id.insert(
        BLANK_TEMPLATE_ID.into(),
        ProjectTemplateInfo {
            display_name: "Blank".into(),
            id: AVATARS_TEMPLATE_ID.into(),
            unity_versions: unity_versions.into(),
            alcom_template: None,
            available: true,
        },
    );

    // then ALCOM templates
    for value in templates {
        let id = value.id.clone().unwrap_or_else(|| {
            format!(
                "{}{}",
                UNNAMED_TEMPLATE_PREFIX,
                uuid::Uuid::new_v4().as_simple()
            )
        });
        template_by_id.insert(
            id.clone(),
            ProjectTemplateInfo {
                display_name: value.display_name.clone(),
                id,
                unity_versions: vec![],
                alcom_template: Some(value),
                available: false,
            },
        );
    }

    let mut keys_to_update = template_by_id.keys().cloned().collect::<HashSet<_>>();

    // Resolve template dependency
    while {
        let mut updated = false;

        keys_to_update.retain(|k| {
            let template = &template_by_id[k];
            if template.available {
                // the template is already avaiable and valid.
                return false;
            }
            let alcom = template.alcom_template.as_ref().unwrap();
            let Some(base) = template_by_id.get(&alcom.base) else {
                // The template will never become available so remove from keys to update
                return false;
            };

            if !base.available {
                // The base template is not available yet. Retry later
                return true;
            }

            // The base template is available! update this template based on the base template

            let unity_versions = if let Some(unity_filter) = &alcom.unity_version {
                base.unity_versions
                    .iter()
                    .copied()
                    .filter(|x| unity_filter.matches(&x.as_semver()))
                    .collect()
            } else {
                base.unity_versions.clone()
            };

            let template_mut = &mut template_by_id[k];
            template_mut.unity_versions = unity_versions;
            template_mut.available = true;

            updated = true;

            false
        });

        updated
    } {}

    Ok(template_by_id.into_values().collect())
}

pub async fn load_alcom_templates(io: &DefaultEnvironmentIo) -> io::Result<Vec<AlcomTemplate>> {
    let path = Path::new("vrc-get/templates");
    let mut dir = io.read_dir(path).await?;
    let mut templates = Vec::new();
    while let Some(entry) = dir.try_next().await? {
        if entry
            .file_name()
            .as_encoded_bytes()
            .ends_with(b".alcomtemplate")
            && entry
                .file_type()
                .await
                .map(|x| x.is_file())
                .unwrap_or(false)
        {
            // The file is alcomtemplate
            let path = path.join(entry.file_name());
            match load_template(io, &path).await {
                Ok(template) => templates.push(template),
                Err(e) => log::warn!(
                    "Error loading template at {path}: {e}",
                    path = path.display()
                ),
            }
        }
    }
    Ok(templates)
}

pub async fn load_template(io: &DefaultEnvironmentIo, path: &Path) -> io::Result<AlcomTemplate> {
    let mut file = io.open(path).await?;
    let mut buffer = vec![];
    file.read_to_end(&mut buffer).await?;
    Ok(parse_alcom_template(&buffer)?)
}

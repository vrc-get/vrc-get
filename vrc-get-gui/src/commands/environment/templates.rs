use crate::commands::prelude::*;
use crate::templates::{AlcomTemplate, parse_alcom_template, serialize_alcom_template};
use crate::utils::trash_delete;
use futures::AsyncWriteExt;
use indexmap::IndexMap;
use itertools::Itertools;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tauri::{State, Window};
use tauri_plugin_dialog::DialogExt;
use vrc_get_vpm::io::{DefaultEnvironmentIo, IoTrait};
use vrc_get_vpm::version::VersionRange;

#[tauri::command]
#[specta::specta]
pub async fn environment_export_template(
    templates: State<'_, TemplatesState>,
    io: State<'_, DefaultEnvironmentIo>,
    window: Window,
    id: String,
) -> Result<(), RustError> {
    let templates = templates.get();
    let Some(template) = templates
        .as_ref()
        .and_then(|x| x.iter().find(|x| x.id == id))
        .take_if(|x| x.source_path.is_some())
    else {
        return Err(RustError::unrecoverable(
            "Template with such id not found (this is bug)",
        ));
    };
    let Some(path) = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_file_name(&template.display_name)
        .add_filter("ALCOM Project Template", &["alcomtemplate"])
        .blocking_save_file()
        .map(|x| x.into_path_buf())
        .transpose()?
    else {
        return Ok(());
    };

    tokio::fs::copy(io.resolve(template.source_path.as_ref().unwrap()), path).await?;

    Ok(())
}

#[derive(Deserialize, Serialize, specta::Type)]
pub struct TauriAlcomTemplate {
    pub display_name: String,
    pub base: String,
    pub unity_version: Option<String>,
    pub vpm_dependencies: IndexMap<String, String>,
    pub unity_packages: Vec<String>,
}

impl From<&AlcomTemplate> for TauriAlcomTemplate {
    fn from(value: &AlcomTemplate) -> Self {
        Self {
            display_name: value.display_name.clone(),
            base: value.base.clone(),
            unity_version: (value.unity_version.as_ref()).map(|x| x.to_string()),
            vpm_dependencies: (value.vpm_dependencies.iter())
                .map(|(pkg, range)| (pkg.clone(), range.to_string()))
                .collect(),
            unity_packages: (value.unity_packages.iter())
                .map(|x| x.to_string_lossy().into_owned())
                .collect(),
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn environment_get_alcom_template(
    templates: State<'_, TemplatesState>,
    id: String,
) -> Result<TauriAlcomTemplate, RustError> {
    match templates
        .get()
        .as_ref()
        .and_then(|x| x.iter().find(|x| x.id == id))
        .and_then(|x| x.alcom_template.as_ref())
    {
        None => Err(RustError::unrecoverable(
            "Template with such id not found (this is bug)",
        )),
        Some(template) => Ok(template.into()),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn environment_pick_unity_package(window: Window) -> Result<Vec<String>, RustError> {
    window
        .dialog()
        .file()
        .set_parent(&window)
        .add_filter("Unity Package", &["unitypackage"])
        .blocking_pick_files()
        .unwrap_or_default()
        .into_iter()
        .map(|x| x.into_path_buf())
        .map_ok(|x| x.to_string_lossy().into_owned())
        .collect::<Result<Vec<_>, _>>()
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn environment_save_template(
    templates: State<'_, TemplatesState>,
    io: State<'_, DefaultEnvironmentIo>,
    id: Option<String>,
    base: String,
    name: String,
    unity_range: String,
    vpm_packages: Vec<(String, String)>,
    unity_packages: Vec<String>,
) -> Result<(), RustError> {
    let template = AlcomTemplate {
        display_name: name.clone(),
        update_date: Some(chrono::Utc::now()),
        id: id
            .as_ref()
            .take_if(|x| !x.starts_with("com.anatawa12.vrc-get.user."))
            .cloned(),
        base,
        unity_version: Some(VersionRange::from_str(&unity_range).map_err(|x| {
            RustError::unrecoverable(format!("Bad Unity Version Range ({unity_range}): {x}"))
        })?),
        vpm_dependencies: vpm_packages
            .into_iter()
            .map(|(pkg, range)| {
                Ok::<_, RustError>((
                    pkg,
                    VersionRange::from_str(&range).map_err(|x| {
                        RustError::unrecoverable(format!("Bad Version Range ({range}): {x}"))
                    })?,
                ))
            })
            .collect::<Result<_, _>>()?,
        unity_packages: unity_packages.into_iter().map(PathBuf::from).collect(),
    };

    let template = serialize_alcom_template(template)
        .map_err(|x| RustError::unrecoverable(format!("Failed to serialize template: {x}")))?;

    if let Some(id) = id {
        // There is id; overwrite existing one
        let templates = templates.get();
        let Some(source_path) = templates
            .as_ref()
            .and_then(|x| x.iter().find(|x| x.id == id))
            .and_then(|x| x.source_path.as_ref())
        else {
            return Err(RustError::unrecoverable(
                "Template with such id not found (this is bug)",
            ));
        };
        io.write_sync(source_path, &template).await?;
    } else {
        // No id; create new one
        save_template_file(&io, &name, &template).await?;
    }

    Ok(())
}

async fn save_template_file(
    io: &DefaultEnvironmentIo,
    name: &str,
    template: &[u8],
) -> io::Result<()> {
    // First, determine file name based on display name
    // Remove Windows Banned Characters
    let file_name = name.replace(['<', '>', ':', '"', '/', '\\', '|', '?', '*'], "");
    // Trim to 50 codepoints.
    // We choose 50 codepoints since 50 codepoints will never exceed 200 bytes in UTF8, and want to be below 200 bytes
    let file_name = {
        let mut chars = file_name.chars();
        // skip (up to) 50 elements
        for _ in 0..100 {
            chars.next();
        }
        // we remove last chars.as_str().len() bytes
        let remainder = chars.as_str().len();
        &file_name[..file_name.len() - remainder]
    };
    // Trim to remove trailing whitesspaces
    let file_name = file_name.trim();
    // Remove trailing '.'s since it's ambigous to extension separator '.'
    let file_name = file_name.trim_end_matches('.');
    // We now have base file name!

    let mut file = 'create_file: {
        let template_dir = Path::new("vrc-get/templates");
        io.create_dir_all(template_dir).await?;
        let extension = "alcomtemplate";
        // first, try original name
        if let Ok(file) = io
            .create_new(&template_dir.join(file_name).with_extension(extension))
            .await
        {
            break 'create_file file;
        }
        // Then, try _numbers up to 10
        for i in 1..=10 {
            if let Ok(file) = io
                .create_new(
                    &template_dir
                        .join(format!("{file_name}_{i}"))
                        .with_extension(extension),
                )
                .await
            {
                break 'create_file file;
            }
        }
        // Finally, try random instead of file name
        io.create_new(
            &template_dir
                .join(uuid::Uuid::new_v4().simple().to_string())
                .with_extension(extension),
        )
        .await?
    };
    file.write_all(template).await?;
    file.flush().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn environment_remove_template(
    templates: State<'_, TemplatesState>,
    io: State<'_, DefaultEnvironmentIo>,
    id: String,
) -> Result<(), RustError> {
    match templates
        .get()
        .as_ref()
        .and_then(|x| x.iter().find(|x| x.id == id))
        .take_if(|x| x.alcom_template.is_some())
        .take_if(|x| x.source_path.is_some())
    {
        None => Err(RustError::unrecoverable(
            "Template with such id not found (this is bug)",
        )),
        Some(template) => {
            let template = io.resolve(template.source_path.as_ref().unwrap());
            if let Err(err) = trash_delete(template.clone()).await {
                error!("failed to remove template: {err}");
            } else {
                info!(
                    "removed template directory: {path}",
                    path = template.display()
                );
            }
            Ok(())
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn environment_import_template(
    window: Window,
    io: State<'_, DefaultEnvironmentIo>,
) -> Result<usize, RustError> {
    let templates = window
        .dialog()
        .file()
        .set_parent(&window)
        .add_filter("ALCOM Project Template", &["alcomtemplate"])
        .blocking_pick_files()
        .unwrap_or_default()
        .into_iter()
        .map(|x| x.into_path_buf())
        .collect::<Result<Vec<_>, _>>()?;

    Ok(import_templates(&io, &templates).await)
}

pub async fn import_templates(io: &DefaultEnvironmentIo, templates: &[PathBuf]) -> usize {
    let mut imported = 0;

    for template in templates {
        let json = match tokio::fs::read(&template).await {
            Ok(json) => json,
            Err(e) => {
                log::error!(
                    "failed to load file: {}: {e}",
                    template.file_name().unwrap().to_string_lossy()
                );
                continue;
            }
        };
        let parsed = match parse_alcom_template(&json) {
            Ok(parsed) => parsed,
            Err(e) => {
                log::error!(
                    "Invalid template: {}: {e}",
                    template.file_name().unwrap().to_string_lossy()
                );
                continue;
            }
        };
        match save_template_file(io, &parsed.display_name, &json).await {
            Ok(()) => {}
            Err(e) => {
                log::error!(
                    "Failed to save imported template: {}: {e}",
                    template.file_name().unwrap().to_string_lossy()
                );
                continue;
            }
        };
        imported += 1;
    }

    imported
}

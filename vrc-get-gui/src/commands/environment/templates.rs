use crate::commands::prelude::*;
use tauri::{State, Window};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
#[specta::specta]
pub async fn environment_export_template(
    templates: State<'_, TemplatesState>,
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

    tokio::fs::copy(template.source_path.as_ref().unwrap(), path).await?;

    Ok(())
}

use futures::{AsyncReadExt, TryStreamExt};
use std::io;
use std::path::Path;
use vrc_get_vpm::io::{DefaultEnvironmentIo, DirEntry, IoTrait};

pub use alcom_template::*;

pub mod alcom_template;

include!(concat!(env!("OUT_DIR"), "/templates.rs"));

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

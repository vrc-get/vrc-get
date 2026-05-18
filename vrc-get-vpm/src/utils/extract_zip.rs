use crate::io;
use crate::io::SeekFrom;
use crate::io::{DefaultProjectIo, IoTrait};
use crate::utils::MapResultExt;
use async_zip::base::read::seek::ZipFileReader;
use futures::prelude::*;
use log::trace;
use std::path::{Component, Path};

pub(crate) async fn extract_zip(
    mut zip_file: impl AsyncBufRead + AsyncSeek + Unpin,
    io: &DefaultProjectIo,
    dest_folder: &Path,
) -> io::Result<()> {
    // extract zip file
    zip_file.seek(SeekFrom::Start(0)).await?;

    let mut zip_reader = ZipFileReader::new(zip_file).await.err_mapped()?;
    for i in 0..zip_reader.file().entries().len() {
        let entry = &zip_reader.file().entries()[i];
        let Some(filename) = entry.filename().as_str().ok() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "path in zip file is not utf8".to_string(),
            ));
        };
        let filename = fix_path_separator(filename);
        let filename = filename.as_ref();
        if !is_complete_relative(filename.as_ref()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("directory traversal detected: {filename}"),
            ));
        }

        let path = dest_folder.join(filename);
        if filename.ends_with('/') {
            // if it's directory, just create directory
            io.create_dir_all(path.as_ref()).await?;
        } else {
            let mut reader = zip_reader.reader_without_entry(i).await.err_mapped()?;
            io.create_dir_all(path.parent().unwrap()).await?;
            let mut dest_file = io.create(path.as_ref()).await?;
            io::copy(&mut reader, &mut dest_file).await?;
            dest_file.flush().await?;
        }
    }

    Ok(())
}

fn fix_path_separator(p: &str) -> std::borrow::Cow<'_, str> {
    if cfg!(windows) {
        // On windows Path struct accepts both '/' and '\' as separator so we don't need to convert separator
        std::borrow::Cow::Borrowed(p)
    } else if !p.contains('\\') {
        // If the path does not contain '\\' we don't need to replace path separators
        std::borrow::Cow::Borrowed(p)
    } else {
        // The path contains '\\', we should replace with '/'
        trace!("fixing '\\' with '/' in path {p:?}");
        std::borrow::Cow::Owned(p.replace('\\', "/"))
    }
}

fn is_complete_relative(path: &Path) -> bool {
    for x in path.components() {
        match x {
            Component::Prefix(_) => return false,
            Component::RootDir => return false,
            Component::ParentDir => return false,
            Component::CurDir => {}
            Component::Normal(_) => {}
        }
    }
    true
}

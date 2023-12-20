use crate::utils::MapResultExt;
use async_zip::tokio::read::seek::ZipFileReader;
use std::io;
use std::io::SeekFrom;
use std::path::{Component, Path};
use tokio::fs::{create_dir_all, File};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};

pub(crate) async fn extract_zip(mut zip_file: File, dest_folder: &Path) -> io::Result<()> {
    // extract zip file
    zip_file.seek(SeekFrom::Start(0)).await?;

    let mut zip_reader = ZipFileReader::new(zip_file.compat()).await.err_mapped()?;
    for i in 0..zip_reader.file().entries().len() {
        let entry = &zip_reader.file().entries()[i];
        let Some(filename) = entry.filename().as_str().ok() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "path in zip file is not utf8".to_string(),
            ));
        };
        if !is_complete_relative(Path::new(filename)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("directory traversal detected: {}", filename),
            ));
        }

        let path = dest_folder.join(filename);
        if filename.ends_with('/') {
            // if it's directory, just create directory
            create_dir_all(path).await?;
        } else {
            let reader = zip_reader.reader_without_entry(i).await.err_mapped()?;
            create_dir_all(path.parent().unwrap()).await?;
            let mut dest_file = File::create(path).await?;
            tokio::io::copy(&mut reader.compat(), &mut dest_file).await?;
            dest_file.flush().await?;
        }
    }

    Ok(())
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

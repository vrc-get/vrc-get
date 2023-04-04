use std::io;
use std::io::SeekFrom;
use std::path::{Component, Path};
use futures::TryStreamExt;
use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::fs::{create_dir_all, File, OpenOptions, remove_dir_all};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use crate::vpm::structs::package::PackageJson;
use crate::vpm::try_open_file;
use crate::vpm::utils::MapResultExt;

pub(crate) async fn add_package(
    global_dir: &Path,
    http: Option<&Client>,
    package: &PackageJson,
    target_packages_folder: &Path,
) -> io::Result<()> {
    log::debug!("adding package {}", package.name);
    let zip_file_name = format!("vrc-get-{}-{}.zip", &package.name, &package.version);
    let zip_path = {
        let mut building = global_dir.to_owned();
        building.push("Repos");
        building.push(&package.name);
        create_dir_all(&building).await?;
        building.push(&zip_file_name);
        building
    };
    let sha_path = zip_path.with_extension("zip.sha256");
    let dest_folder = target_packages_folder.join(&package.name);

    fn parse_hex(hex: [u8; 256 / 4]) -> Option<[u8; 256 / 8]> {
        let mut result = [0u8; 256 / 8];
        for i in 0..(256 / 8) {
            let upper = match hex[i * 2 + 0] {
                c @ b'0'..=b'9' => c - b'0',
                c @ b'a'..=b'f' => c - b'a' + 10,
                c @ b'A'..=b'F' => c - b'A' + 10,
                _ => return None,
            };
            let lower = match hex[i * 2 + 1] {
                c @ b'0'..=b'9' => c - b'0',
                c @ b'a'..=b'f' => c - b'a' + 10,
                c @ b'A'..=b'F' => c - b'A' + 10,
                _ => return None,
            };
            result[i] = upper << 4 | lower;
        }
        Some(result)
    }

    fn to_hex(data: &[u8]) -> String {
        static HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
        let mut result = vec![0u8; data.len() * 2];
        for i in 0..data.len() {
            result[i * 2 + 0] = HEX_CHARS[((data[i] >> 4) & 0xf) as usize];
            result[i * 2 + 1] = HEX_CHARS[((data[i] >> 0) & 0xf) as usize];
        }
        unsafe { String::from_utf8_unchecked(result) }
    }

    async fn try_cache(zip_path: &Path, sha_path: &Path) -> Option<File> {
        let mut cache_file = try_open_file(&zip_path).await.ok()??;
        let mut sha_file = try_open_file(&sha_path).await.ok()??;

        let mut buf = [0u8; 256 / 4];
        sha_file.read_exact(&mut buf).await.ok()?;

        let hex = parse_hex(buf)?;

        let mut sha256 = Sha256::default();
        let mut buffer = [0u8; 1024 * 4];

        // process sha256
        loop {
            match cache_file.read(&mut buffer).await {
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
                Ok(0) => break,
                Ok(size) => sha256.update(&buffer[0..size]),
            }
        }

        drop(buffer);

        let hash = sha256.finalize();
        let hash = &hash[..];
        if hash != &hex[..] {
            return None;
        }

        cache_file.seek(SeekFrom::Start(0)).await.ok()?;

        Some(cache_file)
    }

    let zip_file = if let Some(cache_file) = try_cache(&zip_path, &sha_path).await {
        cache_file
    } else {
        // file not found: err
        let mut cache_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&zip_path)
            .await?;

        let mut sha256 = Sha256::default();

        let Some(http) = http else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Offline mode"))
        };

        let mut stream = http
            .get(&package.url)
            .send()
            .await
            .err_mapped()?
            .error_for_status()
            .err_mapped()?
            .bytes_stream();

        while let Some(data) = stream.try_next().await.err_mapped()? {
            sha256.update(&data);
            cache_file.write_all(&data).await?;
        }

        cache_file.flush().await?;
        cache_file.seek(SeekFrom::Start(0)).await?;

        // write sha file
        let mut sha_file = File::create(&sha_path).await?;
        let hash_hex = to_hex(&sha256.finalize()[..]);
        let sha_file_content = format!("{} {}\n", hash_hex, zip_file_name);
        sha_file.write_all(sha_file_content.as_bytes()).await?;
        sha_file.flush().await?;
        drop(sha_file);

        cache_file
    };

    // remove dest folder before extract if exists
    remove_dir_all(&dest_folder).await.ok();

    // extract zip file
    let mut zip_reader = async_zip::tokio::read::seek::ZipFileReader::new(zip_file)
        .await
        .err_mapped()?;
    for i in 0..zip_reader.file().entries().len() {
        let entry = zip_reader.file().entries()[i].entry();
        let path = dest_folder.join(entry.filename());
        if !check_path(Path::new(entry.filename())) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("directory traversal detected: {}", path.display()),
            )
                .into());
        }
        if entry.dir() {
            // if it's directory, just create directory
            create_dir_all(path).await?;
        } else {
            let mut reader = zip_reader.entry(i).await.err_mapped()?;
            create_dir_all(path.parent().unwrap()).await?;
            let mut dest_file = File::create(path).await?;
            tokio::io::copy(&mut reader, &mut dest_file).await?;
            dest_file.flush().await?;
        }
    }

    Ok(())
}

fn check_path(path: &Path) -> bool {
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

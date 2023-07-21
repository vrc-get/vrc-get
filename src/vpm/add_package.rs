use std::collections::VecDeque;
use std::io;
use std::io::SeekFrom;
use std::path::{Component, Path, PathBuf};
use futures::TryStreamExt;
use indexmap::IndexMap;
use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::fs::{create_dir_all, File, OpenOptions, remove_dir_all};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};
use crate::vpm::structs::package::PackageJson;
use crate::vpm::{PackageInfo, PackageInfoInner, try_open_file};
use crate::vpm::utils::{MapResultExt, parse_hex_256, PathBufExt};

pub(crate) async fn add_package(
    global_dir: &Path,
    http: Option<&Client>,
    package: PackageInfo<'_>,
    target_packages_folder: &Path,
) -> io::Result<()> {
    log::debug!("adding package {}", package.name());
    match package.inner {
        PackageInfoInner::Remote(json, user_repo) => {
            add_remote_package(global_dir, http, json, user_repo.headers(), target_packages_folder).await
        }
        PackageInfoInner::Local(json, path) => {
            add_local_package(path, &json.name, target_packages_folder).await
        }
    }
}

async fn add_remote_package(
    global_dir: &Path,
    http: Option<&Client>,
    package: &PackageJson,
    headers: &IndexMap<String, String>,
    target_packages_folder: &Path,
) -> io::Result<()> {
    let zip_file_name = format!("vrc-get-{}-{}.zip", &package.name, &package.version);
    let zip_path = global_dir.to_owned()
        .joined("Repos")
        .joined(&package.name)
        .joined(&zip_file_name);
    create_dir_all(zip_path.parent().unwrap()).await?;
    let sha_path = zip_path.with_extension("zip.sha256");
    let dest_folder = target_packages_folder.join(&package.name);

    // TODO: set sha256 when zipSHA256 is documented
    let zip_file = if let Some(cache_file) = try_cache(&zip_path, &sha_path, None).await {
        cache_file
    } else {
        download_zip(http, headers, &zip_path, &sha_path, &zip_file_name, &package.url).await?
    };

    // remove dest folder before extract if exists
    remove_dir_all(&dest_folder).await.ok();

    // extract zip file
    let mut zip_reader = async_zip::tokio::read::seek::ZipFileReader::new(
        zip_file.compat())
        .await
        .err_mapped()?;
    for i in 0..zip_reader.file().entries().len() {
        let entry = zip_reader.file().entries()[i].entry();
        let Some(filename) = entry.filename().as_str().ok() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("path in zip file is not utf8"),
            )
                .into());
        };
        let path = dest_folder.join(filename);
        if !check_path(Path::new(filename)) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("directory traversal detected: {}", path.display()),
            )
                .into());
        }
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

/// Try to load from the zip file
/// 
/// # Arguments 
/// 
/// * `zip_path`: the path to zip file
/// * `sha_path`: the path to sha256 file
/// * `sha256`: sha256 hash if specified
/// 
/// returns: Option<File> readable zip file file or None
/// 
/// # Examples 
/// 
/// ```
/// 
/// ```
async fn try_cache(zip_path: &Path, sha_path: &Path, sha256: Option<&str>) -> Option<File> {
    let mut cache_file = try_open_file(&zip_path).await.ok()??;
    let mut sha_file = try_open_file(&sha_path).await.ok()??;

    let mut buf = [0u8; 256 / 4];
    sha_file.read_exact(&mut buf).await.ok()?;

    let hex = parse_hex_256(buf)?;

    // is stored sha doesn't match sha in repo: current cache is invalid
    if let Some(repo_hash) = sha256.and_then(|s| s.as_bytes().try_into().ok()).and_then(parse_hex_256) {
        if repo_hash != hex {
            return None;
        }
    }

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

/// downloads the zip file from the url to the specified path 
/// 
/// # Arguments 
/// 
/// * `http`: http client. returns error if none
/// * `zip_path`: the path to zip file
/// * `sha_path`: the path to sha256 file
/// * `zip_file_name`: the name of zip file. will be used in the sha file
/// * `url`: url to zip file
/// 
/// returns: Result<File, Error> the readable zip file.
async fn download_zip(
    http: Option<&Client>,
    headers: &IndexMap<String, String>,
    zip_path: &Path,
    sha_path: &Path,
    zip_file_name: &str,
    url: &str,
) -> io::Result<File> {
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

    let mut request = http
        .get(url);

    for (name, header) in headers {
        request = request.header(name, header);
    }

    let mut stream = 
        request
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

    Ok(cache_file)
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

async fn add_local_package(package: &Path, name: &str, target_packages_folder: &Path) -> io::Result<()> {
    let dest_folder = target_packages_folder.join(name);
    remove_dir_all(&dest_folder).await.ok();
    copy_recursive(package.to_owned(), dest_folder).await
}

async fn copy_recursive(src_dir: PathBuf, dst_dir: PathBuf) -> io::Result<()> {
    // TODO: parallelize & speedup
    let mut queue = VecDeque::new();
    queue.push_front((src_dir, dst_dir));

    while let Some((src_dir, dst_dir)) = queue.pop_back() {
        let mut iter = tokio::fs::read_dir(src_dir).await?;
        create_dir_all(&dst_dir).await?;
        while let Some(entry) = iter.next_entry().await? {
            let file_type = entry.file_type().await?;
            let src = entry.path();
            let dst = dst_dir.join(entry.file_name());

            if file_type.is_symlink() {
                // symlink: just copy
                let symlink = tokio::fs::read_link(src).await?;
                if symlink.is_absolute() {
                    return Err(io::Error::new(io::ErrorKind::PermissionDenied, "absolute symlink detected"));
                }

                #[cfg(unix)]
                tokio::fs::symlink(dst, symlink).await?;
                #[cfg(windows)]
                {
                    use std::os::windows::fs::FileTypeExt;
                    if file_type.is_symlink_file() {
                        tokio::fs::symlink_file(dst, symlink).await?;
                    } else {
                        assert!(file_type.is_symlink_dir(), "unknown symlink");
                        tokio::fs::symlink_dir(dst, symlink).await?;
                    }
                }
                #[cfg(not(any(unix, windows)))]
                return Err(io::Error::new(io::ErrorKind::Unsupported, "platform without symlink detected"));
            } else if file_type.is_file() {
                tokio::fs::copy(src, dst).await?;
            } else if file_type.is_dir() {
                //copy_recursive(&src, &dst).await?;
                queue.push_front((src, dst));
            } else {
                panic!("unknown file type: none of file, dir, symlink")
            }
        }
    }

    Ok(())
}

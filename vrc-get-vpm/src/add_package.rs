use crate::structs::package::PackageJson;
use crate::utils::{
    copy_recursive, extract_zip, parse_hex_256, MapResultExt, PathBufExt, Sha256AsyncWrite,
};
use crate::{PackageInfo, PackageInfoInner};
use futures::{StreamExt, TryStreamExt};
use indexmap::IndexMap;
use reqwest::{Client, Response};
use std::io;
use std::io::SeekFrom;
use std::path::Path;
use tokio::fs::{create_dir_all, remove_dir_all, File, OpenOptions};
use tokio::io::{AsyncReadExt as _, AsyncSeekExt, AsyncWriteExt};
use tokio_util::compat::FuturesAsyncReadCompatExt;

pub(crate) async fn add_package(
    global_dir: &Path,
    http: Option<&Client>,
    package: PackageInfo<'_>,
    target_packages_folder: &Path,
) -> io::Result<()> {
    log::debug!("adding package {}", package.name());
    match package.inner {
        PackageInfoInner::Remote(json, user_repo) => {
            add_remote_package(
                global_dir,
                http,
                json,
                user_repo.headers(),
                target_packages_folder,
            )
            .await
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
    let zip_path = global_dir
        .to_owned()
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
        download_zip(
            http,
            headers,
            &zip_path,
            &sha_path,
            &zip_file_name,
            &package.url,
        )
        .await?
    };

    // remove dest folder before extract if exists
    remove_dir_all(&dest_folder).await.ok();

    extract_zip(zip_file, &dest_folder).await?;

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
    let mut cache_file = File::open(zip_path).await.ok()?;

    let mut buf = [0u8; 256 / 4];
    File::open(sha_path)
        .await
        .ok()?
        .read_exact(&mut buf)
        .await
        .ok()?;

    let hex = parse_hex_256(buf)?;

    // is stored sha doesn't match sha in repo: current cache is invalid
    if let Some(repo_hash) = sha256
        .and_then(|s| s.as_bytes().try_into().ok())
        .and_then(parse_hex_256)
    {
        if repo_hash != hex {
            return None;
        }
    }

    let mut hasher = Sha256AsyncWrite::new(tokio::io::sink());

    tokio::io::copy(&mut cache_file, &mut hasher).await.ok()?;

    let hash = &hasher.finalize().1[..];
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
    let Some(http) = http else {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Offline mode"));
    };

    // file not found: err
    let cache_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&zip_path)
        .await?;

    let mut request = http.get(url);

    for (name, header) in headers {
        request = request.header(name, header);
    }

    let mut response = request
        .send()
        .await
        .and_then(Response::error_for_status)
        .err_mapped()?
        .bytes_stream()
        .map(|x| x.err_mapped())
        .into_async_read()
        .compat();

    let mut writer = Sha256AsyncWrite::new(cache_file);
    tokio::io::copy(&mut response, &mut writer).await?;

    let (mut cache_file, hash) = writer.finalize();

    cache_file.flush().await?;
    cache_file.seek(SeekFrom::Start(0)).await?;

    // write sha file
    tokio::fs::write(
        &sha_path,
        format!("{} {}\n", hex::encode(&hash[..]), zip_file_name),
    )
    .await?;

    Ok(cache_file)
}

async fn add_local_package(
    package: &Path,
    name: &str,
    target_packages_folder: &Path,
) -> io::Result<()> {
    let dest_folder = target_packages_folder.join(name);
    remove_dir_all(&dest_folder).await.ok();
    copy_recursive(package.to_owned(), dest_folder).await
}

use crate::environment::REPO_CACHE_FOLDER;
use crate::io::{DefaultEnvironmentIo, DefaultProjectIo, IoTrait, TokioFile};
use crate::repository::LocalCachedRepository;
use crate::traits::AbortCheck;
use crate::utils::Sha256AsyncWrite;
use crate::{HttpClient, PackageInfo, PackageManifest, io};
use futures::prelude::*;
use hex::FromHex;
use indexmap::IndexMap;
use log::{debug, error};
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::pin::pin;
use url::Url;

pub struct PackageInstaller<'a, T: HttpClient> {
    pub(super) io: &'a DefaultEnvironmentIo,
    pub(super) http: Option<&'a T>,
}

impl<'a, T: HttpClient> PackageInstaller<'a, T> {
    pub fn new(io: &'a DefaultEnvironmentIo, http: Option<&'a T>) -> Self {
        Self { io, http }
    }
}

impl<T: HttpClient> crate::PackageInstaller for PackageInstaller<'_, T> {
    async fn install_package(
        &self,
        io: &DefaultProjectIo,
        package: PackageInfo<'_>,
        abort: &AbortCheck,
    ) -> io::Result<()> {
        abort.check()?;
        use crate::PackageInfoInner;
        log::debug!("adding package {}", package.name());
        let dest_folder = PathBuf::from(format!("Packages/{}", package.name()));
        match package.inner {
            PackageInfoInner::Remote(package, user_repo) => {
                let zip_file = get_package(self.io, self.http, user_repo, package).await?;

                // downloading may take a long time, so check abort again
                abort.check()?;

                let zip_file = io::BufReader::new(zip_file);

                debug!(
                    "Extracting zip file for {}@{}",
                    package.name(),
                    package.version()
                );
                // remove dest folder before extract if exists
                if let Err(e) = crate::utils::extract_zip(zip_file, io, &dest_folder).await {
                    // if an error occurs, try to remove the dest folder
                    log::debug!(
                        "Error occurred while extracting zip file for {}@{}: {e}",
                        package.name(),
                        package.version(),
                    );
                    let _ = io.remove_dir_all(&dest_folder).await;
                    return Err(e);
                }
                debug!(
                    "Extracted zip file for {}@{}",
                    package.name(),
                    package.version()
                );

                Ok(())
            }
            PackageInfoInner::Local(_, path) => {
                crate::utils::copy_recursive(self.io, path.into(), io, dest_folder).await?;
                Ok(())
            }
        }
    }
}

async fn get_package<T: HttpClient>(
    io: &DefaultEnvironmentIo,
    http: Option<&T>,
    repository: &LocalCachedRepository,
    package: &PackageManifest,
) -> io::Result<TokioFile> {
    let zip_file_name = format!("vrc-get-{}-{}.zip", &package.name(), package.version());
    let zip_path = PathBuf::from(format!(
        "{REPO_CACHE_FOLDER}/{}/{}",
        package.name(),
        &zip_file_name
    ));
    let sha_path = zip_path.with_extension("zip.sha256");

    if let Some(cache_file) =
        try_load_package_cache(io, &zip_path, &sha_path, package.zip_sha_256()).await
    {
        debug!("using cache for {}@{}", package.name(), package.version());
        Ok(cache_file)
    } else {
        io.create_dir_all(zip_path.parent().unwrap()).await?;

        let new_headers = IndexMap::from_iter(
            (repository
                .headers()
                .iter()
                .map(|(k, v)| (k.as_ref(), v.as_ref())))
            .chain(
                package
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.as_ref(), v.as_ref())),
            ),
        );

        let (zip_file, zip_hash) = download_package_zip(
            http,
            io,
            &new_headers,
            &zip_path,
            &sha_path,
            &zip_file_name,
            package.url().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "URL field of the package.json in the repository empty",
                )
            })?,
        )
        .await?;

        if let Some(repo_hash) = package
            .zip_sha_256()
            .and_then(|x| <[u8; 256 / 8] as FromHex>::from_hex(x).ok())
            && repo_hash != zip_hash
        {
            error!(
                "Package hash mismatched! This will be hard error in the future!: {} v{}",
                package.name(),
                package.version()
            );
            //return None;
        }

        Ok(zip_file)
    }
}

/// Try to load from the zip file
///
/// # Arguments
///
/// * `zip_path`: the path to zip file
/// * `sha_path`: the path to sha256 file
/// * `sha256`: sha256 hash if specified
///
/// returns: Option<File> readable zip file or None
async fn try_load_package_cache(
    io: &DefaultEnvironmentIo,
    zip_path: &Path,
    sha_path: &Path,
    sha256: Option<&str>,
) -> Option<TokioFile> {
    let mut cache_file = io.open(zip_path).await.ok()?;

    let mut buf = [0u8; 256 / 4];
    io.open(sha_path)
        .await
        .ok()?
        .read_exact(&mut buf)
        .await
        .ok()?;

    let hex: [u8; 256 / 8] = FromHex::from_hex(buf).ok()?;

    // if stored sha doesn't match sha in repo: current cache is invalid
    if let Some(repo_hash) = sha256.and_then(|x| <[u8; 256 / 8] as FromHex>::from_hex(x).ok())
        && repo_hash != hex
    {
        return None;
    }

    let mut hasher = Sha256AsyncWrite::new(io::sink());

    io::copy(&mut cache_file, &mut hasher).await.ok()?;

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
async fn download_package_zip(
    http: Option<&impl HttpClient>,
    io: &DefaultEnvironmentIo,
    headers: &IndexMap<&str, &str>,
    zip_path: &Path,
    sha_path: &Path,
    zip_file_name: &str,
    url: &Url,
) -> io::Result<(TokioFile, [u8; 256 / 8])> {
    let Some(http) = http else {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Offline mode"));
    };

    // file not found: err
    let cache_file = io.create(zip_path).await?;

    debug!("Download started for {url}");
    let mut response = pin!(http.get(url, headers).await?);

    let mut writer = Sha256AsyncWrite::new(cache_file);
    io::copy(&mut response, &mut writer).await?;
    debug!("finished downloading {url}");

    let (mut cache_file, hash) = writer.finalize();
    let hash: [u8; 256 / 8] = hash.into();

    cache_file.flush().await?;
    cache_file.seek(SeekFrom::Start(0)).await?;

    // write sha file
    io.write(
        sha_path,
        format!("{} {zip_file_name}\n", hex::encode(&hash[..])).as_bytes(),
    )
    .await?;

    Ok((cache_file, hash))
}

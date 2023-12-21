//! The vpm client library.
//!
//! TODO: documentation

#![forbid(unsafe_code)]

use std::io;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde_json::{Map, Value};
use tokio::fs::{create_dir_all, File};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use url::Url;

use structs::package::PartialUnityVersion;
use version::{ReleaseType, UnityVersion, Version, VersionRange};

pub mod environment;
mod repo_holder;
pub mod repository;
mod structs;
mod traits;
pub mod unity_project;
mod utils;
pub mod version;

type JsonMap = Map<String, Value>;

use crate::repository::RemoteRepository;
pub use environment::Environment;
pub use environment::PackageSelector;
pub use unity_project::UnityProject;

use crate::repository::local::LocalCachedRepository;
pub use traits::HttpClient;
pub use traits::PackageCollection;
pub use traits::RemotePackageDownloader;

pub use structs::package::PackageJson;
pub use structs::setting::UserRepoSetting;

#[derive(Copy, Clone)]
pub struct PackageInfo<'a> {
    inner: PackageInfoInner<'a>,
}

#[derive(Copy, Clone)]
enum PackageInfoInner<'a> {
    Remote(&'a PackageJson, &'a LocalCachedRepository),
    Local(&'a PackageJson, &'a Path),
}

impl<'a> PackageInfo<'a> {
    pub fn package_json(self) -> &'a PackageJson {
        // this match will be removed in the optimized code because package.json is exists at first
        match self.inner {
            PackageInfoInner::Remote(pkg, _) => pkg,
            PackageInfoInner::Local(pkg, _) => pkg,
        }
    }

    pub(crate) fn remote(json: &'a PackageJson, repo: &'a LocalCachedRepository) -> Self {
        Self {
            inner: PackageInfoInner::Remote(json, repo),
        }
    }

    pub(crate) fn local(json: &'a PackageJson, path: &'a Path) -> Self {
        Self {
            inner: PackageInfoInner::Local(json, path),
        }
    }

    #[allow(unused)]
    pub fn is_remote(self) -> bool {
        matches!(self.inner, PackageInfoInner::Remote(_, _))
    }

    #[allow(unused)]
    pub fn is_local(self) -> bool {
        matches!(self.inner, PackageInfoInner::Local(_, _))
    }

    pub fn name(self) -> &'a str {
        &self.package_json().name
    }

    pub fn version(self) -> &'a Version {
        &self.package_json().version
    }

    pub fn vpm_dependencies(self) -> &'a IndexMap<String, VersionRange> {
        &self.package_json().vpm_dependencies
    }

    pub fn legacy_packages(self) -> &'a Vec<String> {
        &self.package_json().legacy_packages
    }

    pub fn unity(self) -> Option<&'a PartialUnityVersion> {
        self.package_json().unity.as_ref()
    }

    pub fn is_yanked(self) -> bool {
        is_truthy(self.package_json().yanked.as_ref())
    }
}

fn is_truthy(value: Option<&Value>) -> bool {
    // see https://developer.mozilla.org/en-US/docs/Glossary/Falsy
    match value {
        Some(Value::Null) => false,
        None => false,
        Some(Value::Bool(false)) => false,
        // No NaN in json
        Some(Value::Number(num)) if num.as_f64() == Some(0.0) => false,
        Some(Value::String(s)) if s.is_empty() => false,
        _ => true,
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum PreDefinedRepoSource {
    Official,
    Curated,
}

impl PreDefinedRepoSource {
    pub fn file_name(self) -> &'static str {
        match self {
            PreDefinedRepoSource::Official => "vrc-official.json",
            PreDefinedRepoSource::Curated => "vrc-curated.json",
        }
    }
    pub fn url(self) -> Url {
        match self {
            PreDefinedRepoSource::Official => {
                Url::parse("https://packages.vrchat.com/official?download").unwrap()
            }
            PreDefinedRepoSource::Curated => {
                Url::parse("https://packages.vrchat.com/curated?download").unwrap()
            }
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            PreDefinedRepoSource::Official => "Official",
            PreDefinedRepoSource::Curated => "Curated",
        }
    }
}

#[derive(Clone)]
#[non_exhaustive]
pub enum RepoSource {
    PreDefined(PreDefinedRepoSource, Url, PathBuf),
    UserRepo(UserRepoSetting),
}

static DEFINED_REPO_SOURCES: &[PreDefinedRepoSource] = &[
    PreDefinedRepoSource::Official,
    PreDefinedRepoSource::Curated,
];

async fn update_from_remote(
    client: &impl HttpClient,
    path: &Path,
    repo: &mut LocalCachedRepository,
) {
    let Some(remote_url) = repo.url().map(|x| x.to_owned()) else {
        return;
    };

    let etag = repo.vrc_get.as_ref().map(|x| x.etag.as_str());
    match RemoteRepository::download_with_etag(client, &remote_url, repo.headers(), etag).await {
        Ok(None) => log::debug!("cache matched downloading {}", remote_url),
        Ok(Some((remote_repo, etag))) => {
            repo.set_repo(remote_repo);

            // set etag
            if let Some(etag) = etag {
                repo.vrc_get.get_or_insert_with(Default::default).etag = etag;
            } else if let Some(x) = repo.vrc_get.as_mut() {
                x.etag.clear()
            }
        }
        Err(e) => {
            log::error!("fetching remote repo '{}': {}", remote_url, e);
        }
    }

    match write_repo(path, repo).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("writing local repo '{}': {}", path.display(), e);
        }
    }
}

async fn write_repo(path: &Path, repo: &LocalCachedRepository) -> io::Result<()> {
    create_dir_all(path.parent().unwrap()).await?;
    let mut file = File::create(path).await?;
    file.write_all(&to_json_vec(repo)?).await?;
    file.flush().await?;
    Ok(())
}

fn unity_compatible(package: &PackageInfo, unity: UnityVersion) -> bool {
    fn is_vrcsdk_for_2019(version: &Version) -> bool {
        version.major == 3 && version.minor <= 4
    }

    fn is_resolver_for_2019(version: &Version) -> bool {
        version.major == 0 && version.minor == 1 && version.patch <= 26
    }

    match package.name() {
        "com.vrchat.avatars" | "com.vrchat.worlds" | "com.vrchat.base"
            if is_vrcsdk_for_2019(package.version()) =>
        {
            // this version of VRCSDK is only for unity 2019 so for other version(s) of unity, it's not satisfied.
            unity.major() == 2019
        }
        "com.vrchat.core.vpm-resolver" if is_resolver_for_2019(package.version()) => {
            // this version of Resolver is only for unity 2019 so for other version(s) of unity, it's not satisfied.
            unity.major() == 2019
        }
        _ => {
            // otherwice, check based on package info

            if let Some(min_unity) = package.unity() {
                unity >= UnityVersion::new(min_unity.0, min_unity.1, 0, ReleaseType::Alpha, 0)
            } else {
                // if there are no info, satisfies for all unity versions
                true
            }
        }
    }
}

/// open file or returns none if not exists
async fn try_open_file(path: &Path) -> io::Result<Option<File>> {
    match File::open(path).await {
        Ok(file) => Ok(Some(file)),
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

async fn load_json_or_else<T>(
    manifest_path: &Path,
    default: impl FnOnce() -> io::Result<T>,
) -> io::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    match try_open_file(manifest_path).await? {
        Some(file) => {
            let vec = read_to_vec(file).await?;
            let mut slice = vec.as_slice();
            slice = slice.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(slice);
            Ok(serde_json::from_slice(slice)?)
        }
        None => default(),
    }
}

async fn load_json_or_default<T>(manifest_path: &Path) -> io::Result<T>
where
    T: serde::de::DeserializeOwned + Default,
{
    load_json_or_else(manifest_path, || Ok(Default::default())).await
}

async fn read_to_vec(mut read: impl AsyncRead + Unpin) -> io::Result<Vec<u8>> {
    let mut vec = Vec::new();
    read.read_to_end(&mut vec).await?;
    Ok(vec)
}

fn to_json_vec<T>(value: &T) -> serde_json::Result<Vec<u8>>
where
    T: ?Sized + serde::Serialize,
{
    serde_json::to_vec_pretty(value)
}

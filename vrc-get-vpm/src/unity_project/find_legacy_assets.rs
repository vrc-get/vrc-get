use crate::io::BufReader;
use crate::io::ProjectIo;
use crate::utils::walk_dir_relative;
use crate::PackageInfo;
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use hex::FromHex;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::pin::pin;

pub(crate) struct LegacyAssets<'a> {
    pub(crate) files: Vec<(Box<Path>, &'a str)>,
    pub(crate) folders: Vec<(Box<Path>, &'a str)>,
}

pub(crate) async fn collect_legacy_assets<'a>(
    io: &impl ProjectIo,
    packages: &[PackageInfo<'a>],
) -> LegacyAssets<'a> {
    let folders = packages.iter().flat_map(|pkg| {
        let name = pkg.name();
        pkg.package_json()
            .legacy_folders()
            .iter()
            .map(|(path, guid)| {
                DefinedLegacyInfo::new_dir(name, path, guid.as_deref().and_then(Guid::parse))
            })
    });
    let files = packages.iter().flat_map(|pkg| {
        let name = pkg.name();
        pkg.package_json()
            .legacy_files()
            .iter()
            .map(|(path, guid)| {
                DefinedLegacyInfo::new_file(name, path, guid.as_deref().and_then(Guid::parse))
            })
    });
    // I think collecting here is not required for implementing Send for collect_legacy_assets,
    // but the compiler fails so collect it here.
    let assets = folders.chain(files).collect::<Vec<_>>();

    let (mut found_files, mut found_folders, find_guids) =
        find_legacy_assets_by_path(io, assets.into_iter()).await;

    if !find_guids.is_empty() {
        find_legacy_assets_by_guid(io, find_guids, &mut found_files, &mut found_folders).await;
    }

    LegacyAssets {
        files: found_files.into_iter().collect(),
        folders: found_folders.into_iter().collect(),
    }
}

fn valid_path(path: &Path) -> bool {
    // removing folders other than Assets and Packages are not allowed.
    if !path.starts_with("Assets") && !path.starts_with("Packages") {
        return false;
    }

    for x in path.components() {
        match x {
            std::path::Component::Normal(_) => (),
            _ => return false,
        }
    }

    true
}

async fn find_legacy_assets_by_path<'a>(
    io: &impl ProjectIo,
    assets: impl Iterator<Item = DefinedLegacyInfo<'a>>,
) -> (
    HashMap<Box<Path>, &'a str>,
    HashMap<Box<Path>, &'a str>,
    HashMap<Guid, (&'a str, bool)>,
) {
    use LegacySearchResult::*;

    let mut futures = pin!(assets
        .map(|info| async move {
            // some packages uses '/' as path separator.
            let relative_path = PathBuf::from(info.path.replace('\\', "/")).into_boxed_path();
            // for security, deny absolute path.
            if relative_path.is_absolute() {
                return None;
            }
            #[allow(clippy::manual_map)] // it's parallel, not just a if-else
            if valid_path(&relative_path)
                && io
                    .metadata(&relative_path)
                    .await
                    .map(|x| x.is_file() == info.is_file)
                    .unwrap_or(false)
                && check_guid(io, relative_path.as_ref(), info.guid).await
            {
                Some(FoundWithPath(
                    info.package_name,
                    relative_path,
                    info.is_file,
                ))
            } else if let Some(guid) = info.guid {
                Some(SearchWithGuid(info.package_name, guid, info.is_file))
            } else {
                None
            }
        })
        .collect::<FuturesUnordered<_>>());

    let mut found_files = HashMap::new();
    let mut found_folders = HashMap::new();
    let mut find_guids = HashMap::new();

    while let Some(info) = futures.next().await {
        match info {
            Some(FoundWithPath(package_name, relative_path, true)) => {
                found_files.insert(relative_path, package_name);
            }
            Some(FoundWithPath(package_name, relative_path, false)) => {
                found_folders.insert(relative_path, package_name);
            }
            Some(SearchWithGuid(package_name, guid, is_file)) => {
                find_guids.insert(guid, (package_name, is_file));
            }
            None => (),
        }
    }

    (found_files, found_folders, find_guids)
}

async fn check_guid(io: &impl ProjectIo, path: &Path, guid: Option<Guid>) -> bool {
    if let Some(guid) = guid {
        let mut path = OsString::from(path);
        path.push(".meta");
        if let Some(actual_guid) = try_parse_meta(io, path.as_ref()).await {
            return actual_guid == guid;
        }
    }
    true
}

async fn try_parse_meta(io: &impl ProjectIo, path: &Path) -> Option<Guid> {
    let mut file = BufReader::new(io.open(path).await.ok()?);
    let mut buffer = String::new();
    while file.read_line(&mut buffer).await.ok()? != 0 {
        let line = buffer.as_str();
        if let Some(guid) = line.strip_prefix("guid: ") {
            // current line should be line for guid.
            return Guid::parse(guid.trim());
        }

        buffer.clear()
    }
    None
}

async fn find_legacy_assets_by_guid<'a>(
    io: &impl ProjectIo,
    mut find_guids: HashMap<Guid, (&'a str, bool)>,
    found_files: &mut HashMap<Box<Path>, &'a str>,
    found_folders: &mut HashMap<Box<Path>, &'a str>,
) {
    async fn get_guid<IO: ProjectIo>(io: &IO, relative: PathBuf) -> Option<(Guid, bool, PathBuf)> {
        if relative.extension() != Some(OsStr::new("meta")) {
            None
        } else if let Some(guid) = try_parse_meta(io, &relative).await {
            // remove .meta extension
            let mut path = relative;
            path.set_extension("");

            let is_file = io.metadata(&path).await.ok()?.is_file();
            Some((guid, is_file, path))
        } else {
            None
        }
    }

    let mut stream =
        pin!(walk_dir_relative(io, [PathBuf::from("Assets")]).filter_map(|x| get_guid(io, x)));

    while let Some((guid, is_file_actual, relative)) = stream.next().await {
        if let Some(&(package_name, is_file)) = find_guids.get(&guid) {
            if is_file_actual == is_file {
                find_guids.remove(&guid);
                if is_file {
                    found_files.insert(relative.into_boxed_path(), package_name);
                } else {
                    found_folders.insert(relative.into_boxed_path(), package_name);
                }
            }
        }
    }
}

struct DefinedLegacyInfo<'a> {
    package_name: &'a str,
    path: &'a str,
    guid: Option<Guid>,
    is_file: bool,
}

impl<'a> DefinedLegacyInfo<'a> {
    fn new_file(name: &'a str, path: &'a str, guid: Option<Guid>) -> Self {
        Self {
            package_name: name,
            path,
            guid,
            is_file: true,
        }
    }

    fn new_dir(name: &'a str, path: &'a str, guid: Option<Guid>) -> Self {
        Self {
            package_name: name,
            path,
            guid,
            is_file: false,
        }
    }
}

enum LegacySearchResult<'a> {
    FoundWithPath(&'a str, Box<Path>, bool),
    SearchWithGuid(&'a str, Guid, bool),
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
struct Guid([u8; 16]);

impl Guid {
    fn parse(guid: &str) -> Option<Guid> {
        FromHex::from_hex(guid).ok().map(Guid)
    }
}

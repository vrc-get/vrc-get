use crate::io::BufReader;
use crate::io::ProjectIo;
use crate::utils::walk_dir_relative;
use crate::PackageInfo;
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use hex::FromHex;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::pin::pin;

pub(crate) struct LegacyAssets {
    pub(crate) files: Vec<Box<Path>>,
    pub(crate) folders: Vec<Box<Path>>,
}

pub(crate) async fn collect_legacy_assets(
    io: &impl ProjectIo,
    packages: &[PackageInfo<'_>],
) -> LegacyAssets {
    let folders = packages
        .iter()
        .flat_map(|x| x.package_json().legacy_folders())
        .map(|(path, guid)| {
            DefinedLegacyInfo::new_dir(path, guid.as_deref().and_then(Guid::parse))
        });
    let files = packages
        .iter()
        .flat_map(|x| x.package_json().legacy_files())
        .map(|(path, guid)| {
            DefinedLegacyInfo::new_file(path, guid.as_deref().and_then(Guid::parse))
        });
    let assets = folders.chain(files);

    let (mut found_files, mut found_folders, find_guids) =
        find_legacy_assets_by_path(io, assets).await;

    if !find_guids.is_empty() {
        find_legacy_assets_by_guid(io, find_guids, &mut found_files, &mut found_folders).await;
    }

    LegacyAssets {
        files: found_files.into_iter().collect(),
        folders: found_folders.into_iter().collect(),
    }
}

async fn find_legacy_assets_by_path(
    io: &impl ProjectIo,
    assets: impl Iterator<Item = DefinedLegacyInfo<'_>>,
) -> (HashSet<Box<Path>>, HashSet<Box<Path>>, HashMap<Guid, bool>) {
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
            if io
                .metadata(&relative_path)
                .await
                .map(|x| x.is_file() == info.is_file)
                .unwrap_or(false)
            {
                Some(FoundWithPath(relative_path, info.is_file))
            } else if let Some(guid) = info.guid {
                Some(SearchWithGuid(guid, info.is_file))
            } else {
                None
            }
        })
        .collect::<FuturesUnordered<_>>());

    let mut found_files = HashSet::new();
    let mut found_folders = HashSet::new();
    let mut find_guids = HashMap::new();

    while let Some(info) = futures.next().await {
        match info {
            Some(FoundWithPath(relative_path, true)) => {
                found_files.insert(relative_path);
            }
            Some(FoundWithPath(relative_path, false)) => {
                found_folders.insert(relative_path);
            }
            Some(SearchWithGuid(guid, is_file)) => {
                find_guids.insert(guid, is_file);
            }
            None => (),
        }
    }

    (found_files, found_folders, find_guids)
}

async fn try_parse_meta(io: &impl ProjectIo, path: &Path) -> Option<Guid> {
    let mut file = BufReader::new(io.open(&path).await.ok()?);
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

async fn find_legacy_assets_by_guid(
    io: &impl ProjectIo,
    mut find_guids: HashMap<Guid, bool>,
    found_files: &mut HashSet<Box<Path>>,
    found_folders: &mut HashSet<Box<Path>>,
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

    let mut stream = pin!(walk_dir_relative(
        io,
        [PathBuf::from("Packages"), PathBuf::from("Assets")]
    )
    .filter_map(|x| get_guid(io, x)));

    while let Some((guid, is_file_actual, relative)) = stream.next().await {
        if let Some(&is_file) = find_guids.get(&guid) {
            if is_file_actual == is_file {
                find_guids.remove(&guid);
                if is_file {
                    found_files.insert(relative.into_boxed_path());
                } else {
                    found_folders.insert(relative.into_boxed_path());
                }
            }
        }
    }
}

struct DefinedLegacyInfo<'a> {
    path: &'a str,
    guid: Option<Guid>,
    is_file: bool,
}

impl<'a> DefinedLegacyInfo<'a> {
    fn new_file(path: &'a str, guid: Option<Guid>) -> Self {
        Self {
            path,
            guid,
            is_file: true,
        }
    }

    fn new_dir(path: &'a str, guid: Option<Guid>) -> Self {
        Self {
            path,
            guid,
            is_file: false,
        }
    }
}

enum LegacySearchResult {
    FoundWithPath(Box<Path>, bool),
    SearchWithGuid(Guid, bool),
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
struct Guid([u8; 16]);

impl Guid {
    fn parse(guid: &str) -> Option<Guid> {
        FromHex::from_hex(guid).ok().map(Guid)
    }
}

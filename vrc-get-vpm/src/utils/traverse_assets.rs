/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */
use crate::io::{BufReader, DefaultProjectIo, DirEntry, IoTrait};
use crate::utils::walk_dir_relative;
use futures::prelude::*;
use hex::FromHex;
use std::path::{Path, PathBuf};

pub struct UnityAsset {
    pub guid: Guid,
    pub is_file: bool,
    pub path: PathBuf,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct Guid([u8; 16]);

impl Guid {
    pub fn parse(guid: &str) -> Option<Guid> {
        FromHex::from_hex(guid).ok().map(Guid)
    }
}

impl serde::Serialize for Guid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut buffer = [0u8; 32];
        hex::encode_to_slice(self.0, &mut buffer).unwrap();
        serializer.serialize_str(str::from_utf8(&buffer).unwrap())
    }
}

pub fn find_assets(io: &DefaultProjectIo) -> impl Stream<Item = UnityAsset> {
    walk_dir_relative(io, [PathBuf::from("Assets")]).filter_map(
        move |(mut path, entry)| async move {
            if path.extension() == Some("meta".as_ref()) {
                return None;
            }
            let is_file = entry.file_type().await.map(|t| t.is_file()).ok()?;
            let guid;
            {
                let mut meta_path = path;
                meta_path.add_extension("meta");
                guid = try_parse_meta(io, &meta_path).await?;
                meta_path.set_extension("");
                path = meta_path;
            }
            Some(UnityAsset {
                path,
                guid,
                is_file,
            })
        },
    )
}

pub async fn try_parse_meta(io: &DefaultProjectIo, path: &Path) -> Option<Guid> {
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

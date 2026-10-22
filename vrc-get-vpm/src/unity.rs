/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

use crate::io;
use crate::version::UnityVersion;
use std::path::Path;
use std::str::from_utf8;
use tokio::process::Command;

pub async fn call_unity_for_version(path: &Path) -> io::Result<UnityVersion> {
    let output = Command::new(path)
        .args([
            "-version",
            "-batchmode",
            "-quit",
            "-noUpm",
            "-nographics",
            "-projectPath",
            &format!("{}", uuid::Uuid::new_v4()),
            "-logfile",
        ])
        .output()
        .await?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unity exists with non zero value: {}", output.status),
        ));
    }

    let stdout = &output.stdout[..];
    let index = stdout
        .iter()
        .position(|&x| x == b' ')
        .unwrap_or(stdout.len());

    let version = from_utf8(&stdout[..index])
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid version"))?
        .trim();

    let version = UnityVersion::parse(version).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid version: {version}"),
        )
    })?;

    Ok(version)
}

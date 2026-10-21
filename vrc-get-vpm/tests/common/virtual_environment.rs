/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

use crate::common::PackageCollection;
use indexmap::IndexMap;
use serde_json::json;
use std::future::Future;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use vrc_get_vpm::io::{DefaultProjectIo, IoTrait};
use vrc_get_vpm::unity_project::pending_project_changes::Remove;
use vrc_get_vpm::version::{Version, VersionRange};
use vrc_get_vpm::{
    AbortCheck, HttpClient, PackageInfo, PackageInstaller, PackageManifest, UnityProject,
};

pub struct VirtualInstaller {}

impl VirtualInstaller {
    pub fn new() -> Self {
        Self {}
    }
}

impl PackageInstaller for VirtualInstaller {
    fn install_package(
        &self,
        _: &DefaultProjectIo,
        _: PackageInfo<'_>,
        _: &std::path::Path,
        _: &AbortCheck,
    ) -> impl Future<Output = io::Result<()>> {
        std::future::ready(Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "install_package not supported in VirtualEnvironment",
        )))
    }
}

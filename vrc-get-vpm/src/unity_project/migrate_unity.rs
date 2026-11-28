/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

use crate::unity_project::{AddPackageErr, AddPackageOperation};
use crate::utils::{Guid, find_assets, save_json};
use crate::version::UnityVersion;
use crate::{
    PackageCollection, PackageInstaller, UnityProject, VRCHAT_RECOMMENDED_2022_UNITY,
    VRCHAT_RECOMMENDED_6000_0_UNITY, VersionSelector, io,
};
use futures::prelude::*;
use log::warn;
use std::future::ready;

#[non_exhaustive]
#[derive(Debug)]
pub enum MigrateUnityError {
    UnityVersionMismatch(&'static str),
    VpmPackageNotFound(&'static str),
    AddPackageErr(AddPackageErr),
    Io(io::Error),
}

impl std::error::Error for MigrateUnityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MigrateUnityError::AddPackageErr(err) => Some(err),
            MigrateUnityError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl std::fmt::Display for MigrateUnityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrateUnityError::UnityVersionMismatch(expected) => {
                write!(f, "Unity version is not {}", expected)
            }
            MigrateUnityError::VpmPackageNotFound(name) => {
                write!(f, "VPM package {name} not found")
            }
            MigrateUnityError::AddPackageErr(err) => write!(f, "{err}"),
            MigrateUnityError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl From<AddPackageErr> for MigrateUnityError {
    fn from(err: AddPackageErr) -> Self {
        MigrateUnityError::AddPackageErr(err)
    }
}

impl From<io::Error> for MigrateUnityError {
    fn from(err: io::Error) -> Self {
        MigrateUnityError::Io(err)
    }
}

type Result<T = (), E = MigrateUnityError> = std::result::Result<T, E>;

pub struct UnityMigration {
    expected_unity_major: u16,
    expected_unity_minor: Option<u8>,
    expected_unity_major_text: &'static str,
    packages_to_remove: &'static [&'static str],
    unity_version: UnityVersion,
    unity_version_text: &'static str,
    with_upgrade_marker: bool,
}

impl UnityMigration {
    pub const UNITY_2022: &UnityMigration = &UNITY_2022_MIGRATION_STATIC;
    pub const UNITY_6000_0: &UnityMigration = &UNITY_6000_0_MIGRATION_STATIC;

    pub fn unity_version(&self) -> UnityVersion {
        self.unity_version
    }

    pub fn unity_version_text(&self) -> &'static str {
        self.unity_version_text
    }
}

impl UnityProject {
    pub async fn migrate_unity(
        &mut self,
        migration: &UnityMigration,
        collection: &impl PackageCollection,
        installer: &impl PackageInstaller,
    ) -> Result {
        migrate_unity(self, migration, collection, installer).await
    }
}

static UNITY_2022_MIGRATION_STATIC: UnityMigration = UnityMigration {
    // See https://misskey.niri.la/notes/9nod7sk4sr for migration process
    expected_unity_major: 2019,
    expected_unity_minor: None,
    expected_unity_major_text: "2019.x",
    // removes legacy XR packages
    packages_to_remove: &[
        "com.unity.xr.oculus.standalone",
        "com.unity.xr.openvr.standalone",
    ],
    unity_version: VRCHAT_RECOMMENDED_2022_UNITY,
    unity_version_text: "2022",
    with_upgrade_marker: false,
};

static UNITY_6000_0_MIGRATION_STATIC: UnityMigration = UnityMigration {
    expected_unity_major: 2022,
    expected_unity_minor: None,
    expected_unity_major_text: "2022.x",
    packages_to_remove: &["com.unity.ide.vscode"],
    unity_version: VRCHAT_RECOMMENDED_6000_0_UNITY,
    unity_version_text: "6.0",
    with_upgrade_marker: true,
};

const MARKER_FILE_LOCATION: &str = "Library/VRCProjectUpgradeMarker.json";

async fn migrate_unity(
    project: &mut UnityProject,
    migration: &UnityMigration,
    collection: &impl PackageCollection,
    installer: &impl PackageInstaller,
) -> Result {
    if project.unity_version().major() != migration.expected_unity_major
        || (migration.expected_unity_minor)
            .is_some_and(|minor| project.unity_version().minor() != minor)
    {
        return Err(MigrateUnityError::UnityVersionMismatch(
            migration.expected_unity_major_text,
        ));
    }

    // since this command is made for projects with VPM VRCSDK, wan if not
    if !is_vpm_vrcsdk_installed(project) {
        warn!("It looks migrating projects without vpm VRCSDK. this may not intended");
    }

    // remove legacy XR packages
    for package in migration.packages_to_remove {
        project.upm_manifest.remove_dependency(package);
    }

    // upgrade VRCSDK if installed
    let mut packages = vec![];
    let migrating_packages = [
        "com.vrchat.base",
        "com.vrchat.avatars",
        "com.vrchat.worlds",
        "com.vrchat.core.vpm-resolver",
    ];
    for package in migrating_packages {
        if project.get_locked(package).is_some() {
            let unity_version = Some(migration.unity_version);
            let Some(vrcsdk) = collection
                .find_package_by_name(package, VersionSelector::latest_for(unity_version, false))
                .or_else(|| {
                    collection.find_package_by_name(
                        package,
                        VersionSelector::latest_for(unity_version, true),
                    )
                })
            else {
                return Err(MigrateUnityError::VpmPackageNotFound(package));
            };
            packages.push(vrcsdk);
        }
    }

    if !packages.is_empty() {
        // install packages
        let request = project
            .add_package_request(
                collection,
                &packages,
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await?;
        project.apply_pending_changes(installer, request).await?;
    }

    if migration.with_upgrade_marker {
        let io = project.io();

        let scene_list = find_assets(io)
            .filter(|asset| ready(asset.path.extension() == Some("unity".as_ref())))
            .filter(|asset| ready(asset.is_file))
            .map(|asset| asset.guid)
            .collect::<Vec<_>>()
            .await;

        save_json(
            io,
            MARKER_FILE_LOCATION.as_ref(),
            &VRCProjectUpgradeMarker {
                upgraded_from: VRCProjectUpgradeMarkerUnityVersion {
                    full: &project.unity_version().to_string(),
                    major: project.unity_version().major() as u32,
                    minor: project.unity_version().minor() as u32,
                    patch: project.unity_version().revision() as u32,
                },
                wizard_completed: false,
                scene_list,
                upgraded_scenes: None,
            },
        )
        .await?;
    }

    Ok(())
}

fn is_vpm_vrcsdk_installed(project: &UnityProject) -> bool {
    if project.get_locked("com.vrchat.base").is_some()
        || project.get_locked("com.vrchat.avatars").is_some()
        || project.get_locked("com.vrchat.worlds").is_some()
    {
        // VRCSDK is installed
        return true;
    }
    if project.get_locked("com.vrchat.core.vpm-resolver").is_some() {
        // VPM Resolver is installed so It looks it's a vpm project.
        return true;
    }
    // otherwice warn
    false
}

#[derive(serde::Serialize)]
struct VRCProjectUpgradeMarker<'a> {
    #[serde(rename = "upgradedFrom")]
    upgraded_from: VRCProjectUpgradeMarkerUnityVersion<'a>,
    #[serde(rename = "wizardCompleted")]
    wizard_completed: bool, // = false
    #[serde(rename = "sceneList")]
    scene_list: Vec<Guid>,
    #[serde(rename = "upgradedScenes")]
    // Infallible does not implement Serialize. see https://github.com/serde-rs/serde/issues/2740
    upgraded_scenes: Option<()>, // = None (null)
}

#[derive(serde::Serialize)]
struct VRCProjectUpgradeMarkerUnityVersion<'a> {
    full: &'a str,
    major: u32,
    minor: u32,
    patch: u32,
}

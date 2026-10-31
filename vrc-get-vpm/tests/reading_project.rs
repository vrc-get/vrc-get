/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

use crate::common::*;
use vrc_get_vpm::version::{ReleaseType, UnityVersion, Version};

mod common;

#[test]
fn empty_project() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency_range("com.anatawa12.package", "^1.0.0")
            .build()
            .await
            .unwrap();

        assert!(project.unlocked_packages().is_empty());
    })
}

#[test]
fn read_version_name() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency_range("com.anatawa12.package", "^1.0.0")
            .with_unity("2022.3.6f1", "b9e6e7e9fa2d")
            .build()
            .await
            .unwrap();

        assert_eq!(
            project.unity_version(),
            UnityVersion::new(2022, 3, 6, ReleaseType::Normal, 1)
        );
    })
}

#[test]
fn read_package_json_with_bad_url() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_locked("com.anatawa12.package", Version::new(1, 0, 0), &[])
            .add_file(
                "Packages/com.anatawa12.package/package.json",
                r#"{
                    "name": "com.anatawa12.package",
                    "version": "1.0.0",
                    "url": ""
                }"#,
            )
            .build()
            .await
            .unwrap();

        println!("{:?}", project.installed_packages().collect::<Vec<_>>());

        let (_, package_json) = project
            .installed_packages()
            .find(|(name, _)| *name == "com.anatawa12.package")
            .unwrap();
        assert_eq!(package_json.name(), "com.anatawa12.package");
    })
}

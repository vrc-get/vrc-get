/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

use crate::common::*;
use vrc_get_vpm::unity_project::pending_project_changes::RemoveReason;
use vrc_get_vpm::version::Version;

mod common;

#[test]
fn basic_remove() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency("com.anatawa12.gists", Version::new(1, 0, 0))
            .add_locked("com.anatawa12.gists", Version::new(1, 0, 0), &[])
            .build()
            .await
            .unwrap();

        let result = project
            .remove_request(&["com.anatawa12.gists"])
            .await
            .unwrap();

        assert_len!(result.package_changes(), 1);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);

        assert_removed(&result, "com.anatawa12.gists", RemoveReason::Requested);
    })
}

#[test]
fn transitive_unused_remove() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency("com.vrchat.avatars", Version::new(1, 0, 0))
            .add_locked(
                "com.vrchat.avatars",
                Version::new(1, 0, 0),
                &[("com.vrchat.base", "1.0.0")],
            )
            .add_locked("com.vrchat.base", Version::new(1, 0, 0), &[])
            .build()
            .await
            .unwrap();

        let result = project
            .remove_request(&["com.vrchat.avatars"])
            .await
            .unwrap();

        assert_len!(result.package_changes(), 2);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);

        assert_removed(&result, "com.vrchat.avatars", RemoveReason::Requested);
        assert_removed(&result, "com.vrchat.base", RemoveReason::Unused);
    })
}

#[test]
fn do_not_remove_transitively_when_untouched() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency("com.vrchat.avatars", Version::new(1, 0, 0))
            .add_locked(
                "com.vrchat.avatars",
                Version::new(1, 0, 0),
                &[("com.vrchat.base", "1.0.0")],
            )
            .add_locked("com.vrchat.base", Version::new(1, 0, 0), &[])
            .add_locked(
                "com.anatawa12.untouched_library",
                Version::new(1, 0, 0),
                &[],
            )
            .build()
            .await
            .unwrap();

        let result = project
            .remove_request(&["com.vrchat.avatars"])
            .await
            .unwrap();

        assert_len!(result.package_changes(), 2);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);

        assert_removed(&result, "com.vrchat.avatars", RemoveReason::Requested);
        assert_removed(&result, "com.vrchat.base", RemoveReason::Unused);
    })
}

#[test]
fn do_not_remove_dependency_of_locked_only_package() {
    block_on(async {
        // The tool is locked but nobody requires it since the manifest is inconsistent.
        // Both the avatars and the tool require the base, and the base requires the core.
        let project = VirtualProjectBuilder::new()
            .add_dependency("com.vrchat.avatars", Version::new(1, 0, 0))
            .add_locked(
                "com.vrchat.avatars",
                Version::new(1, 0, 0),
                &[("com.vrchat.base", "1.0.0")],
            )
            .add_locked(
                "com.anatawa12.tool",
                Version::new(1, 0, 0),
                &[("com.vrchat.base", "1.0.0")],
            )
            .add_locked(
                "com.vrchat.base",
                Version::new(1, 0, 0),
                &[("com.vrchat.core", "1.0.0")],
            )
            .add_locked("com.vrchat.core", Version::new(1, 0, 0), &[])
            .build()
            .await
            .unwrap();

        let result = project
            .remove_request(&["com.vrchat.avatars"])
            .await
            .unwrap();

        // We keep the tool so the tool needs the base.
        assert_len!(result.package_changes(), 1);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);

        assert_removed(&result, "com.vrchat.avatars", RemoveReason::Requested);
    })
}

#[test]
fn do_not_conflict_with_package_removed_as_unused() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency("com.vrchat.avatars", Version::new(1, 0, 0))
            .add_dependency("com.vrchat.core", Version::new(1, 0, 0))
            .add_locked(
                "com.vrchat.avatars",
                Version::new(1, 0, 0),
                &[("com.vrchat.base", "1.0.0")],
            )
            .add_locked(
                "com.vrchat.base",
                Version::new(1, 0, 0),
                &[("com.vrchat.core", "1.0.0")],
            )
            .add_locked("com.vrchat.core", Version::new(1, 0, 0), &[])
            .build()
            .await
            .unwrap();

        let result = project
            .remove_request(&["com.vrchat.avatars", "com.vrchat.core"])
            .await
            .unwrap();

        // The base requires the core, but the base is removed together since only
        // the avatars requires it. So removing the core doesn't conflict with the base.
        assert_len!(result.package_changes(), 3);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);

        assert_removed(&result, "com.vrchat.avatars", RemoveReason::Requested);
        assert_removed(&result, "com.vrchat.core", RemoveReason::Requested);
        assert_removed(&result, "com.vrchat.base", RemoveReason::Unused);
    })
}

#[test]
fn remove_referenced_legacy_package() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency("com.anatawa12.user", Version::new(1, 0, 0))
            .add_dependency("com.anatawa12.legacy-package", Version::new(1, 0, 0))
            .add_locked(
                "com.anatawa12.user",
                Version::new(1, 0, 0),
                &[("com.anatawa12.legacy-package", "^1.0.0")],
            )
            .add_locked("com.anatawa12.legacy-package", Version::new(1, 0, 0), &[])
            .add_locked("com.anatawa12.package", Version::new(1, 1, 0), &[])
            .add_package_json(
                "com.anatawa12.package",
                r#"
            {
                "name": "com.anatawa12.package",
                "version": "1.1.0",
                "legacyPackages": ["com.anatawa12.legacy-package"]
            }
            "#,
            )
            .build()
            .await
            .unwrap();

        let result = project
            .remove_request(&["com.anatawa12.legacy-package"])
            .await
            .unwrap();

        assert_len!(result.package_changes(), 1);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);

        assert_removed(
            &result,
            "com.anatawa12.legacy-package",
            RemoveReason::Requested,
        );
    })
}

/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

use crate::common::*;
use vrc_get_vpm::PackageManifest;
use vrc_get_vpm::version::Version;

mod common;

#[test]
fn simple_resolve_fully_locked() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency("com.anatawa12.package", Version::new(1, 0, 0))
            .add_locked(
                "com.anatawa12.package",
                Version::new(1, 0, 0),
                &[("com.anatawa12.library", "1.0.0")],
            )
            .add_locked("com.anatawa12.library", Version::new(1, 0, 0), &[])
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.anatawa12.package", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.anatawa12.library", "1.0.0"),
            )
            .add(PackageManifest::new(
                "com.anatawa12.library",
                Version::new(1, 0, 0),
            ))
            .build();

        let result = project.resolve_request(&collection).await.unwrap();

        assert_len!(result.package_changes(), 2);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 0, 0));
        let library = collection.get_package("com.anatawa12.library", Version::new(1, 0, 0));
        assert_install_only(&result, &package);
        assert_install_only(&result, &library);
    })
}

#[test]
fn resolve_ranged() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency_range("com.anatawa12.package", "^1.0.0")
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.anatawa12.package", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.anatawa12.library", "1.0.0"),
            )
            .add(PackageManifest::new(
                "com.anatawa12.library",
                Version::new(1, 0, 0),
            ))
            .build();

        let result = project.resolve_request(&collection).await.unwrap();

        assert_len!(result.package_changes(), 2);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 0, 0));
        let library = collection.get_package("com.anatawa12.library", Version::new(1, 0, 0));
        assert_installing_to_both(&result, &package);
        assert_installing_to_locked_only(&result, &library);
    })
}

#[test]
fn resolve_ranged_to_prerelease_if_no_stable_version() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency_range("com.anatawa12.package", "^1.0.0")
            .build()
            .await
            .unwrap();

        let prerelease = "1.2.0-beta.1".parse::<Version>().unwrap();
        let collection = PackageCollectionBuilder::new()
            .add(PackageManifest::new(
                "com.anatawa12.package",
                prerelease.clone(),
            ))
            .build();

        let result = project.resolve_request(&collection).await.unwrap();

        // The range doesn't have prerelease versions, but a prerelease version is better than nothing.
        let package = collection.get_package("com.anatawa12.package", prerelease);
        assert_installing_to_both(&result, &package);

        assert_len!(result.package_changes(), 1);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);
    })
}

#[test]
fn resolve_prerelease_dependency_to_newest_version() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency(
                "com.anatawa12.package",
                "1.0.0-beta.1".parse::<Version>().unwrap(),
            )
            .build()
            .await
            .unwrap();

        // The collection for tests returns the first match so newer versions come first.
        let newest = "1.1.0-beta.1".parse::<Version>().unwrap();
        let collection = PackageCollectionBuilder::new()
            .add(PackageManifest::new(
                "com.anatawa12.package",
                newest.clone(),
            ))
            .add(PackageManifest::new(
                "com.anatawa12.package",
                Version::new(1, 0, 0),
            ))
            .add(PackageManifest::new(
                "com.anatawa12.package",
                "1.0.0-beta.1".parse::<Version>().unwrap(),
            ))
            .build();

        let result = project.resolve_request(&collection).await.unwrap();

        // The project requires a prerelease version, so we select the newest version
        // even if it's a prerelease version and there is a stable version.
        let package = collection.get_package("com.anatawa12.package", newest);
        assert_installing_to_both(&result, &package);

        assert_len!(result.package_changes(), 1);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);
    })
}

#[test]
fn resolve_dependency_installed_as_unlocked() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency("com.anatawa12.package", Version::new(1, 0, 0))
            .add_file(
                "Packages/unlocked/package.json",
                r#"
            {
                "name": "com.anatawa12.package",
                "version": "1.0.0"
            }
            "#,
            )
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(PackageManifest::new(
                "com.anatawa12.package",
                Version::new(1, 0, 0),
            ))
            .build();

        let result = project.resolve_request(&collection).await.unwrap();

        // The project requires the package to be locked,
        // so we install it even if it's installed as an unlocked package.
        // The project is in an invalid state, so this is the current behavior,
        // not a behavior we guarantee.
        let package = collection.get_package("com.anatawa12.package", Version::new(1, 0, 0));
        assert_installing_to_both(&result, &package);

        assert_len!(result.package_changes(), 1);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);
    })
}

#[test]
fn resolve_dependencies_of_unlocked() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_file(
                "Packages/unlocked/package.json",
                r#"
            {
                "name": "com.anatawa12.package",
                "version": "1.0.0",
                "vpmDependencies": {
                    "com.anatawa12.library": "^1.0.0"
                }
            }
            "#,
            )
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(PackageManifest::new(
                "com.anatawa12.library",
                Version::new(1, 0, 0),
            ))
            .build();

        let result = project.resolve_request(&collection).await.unwrap();

        assert_len!(result.package_changes(), 1);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);

        let library = collection.get_package("com.anatawa12.library", Version::new(1, 0, 0));
        assert_installing_to_locked_only(&result, &library);
    })
}

#[test]
fn resolve_dependencies_of_prerelease_unlocked() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_file(
                "Packages/unlocked/package.json",
                r#"
            {
                "name": "com.anatawa12.package",
                "version": "1.0.0-beta.1",
                "vpmDependencies": {
                    "com.anatawa12.library": "^1.0.0"
                }
            }
            "#,
            )
            .build()
            .await
            .unwrap();

        let prerelease = "1.2.0-beta.1".parse::<Version>().unwrap();
        let collection = PackageCollectionBuilder::new()
            .add(PackageManifest::new(
                "com.anatawa12.library",
                prerelease.clone(),
            ))
            .build();

        let result = project.resolve_request(&collection).await.unwrap();

        // We allow prerelease versions for dependencies of prerelease packages.
        let library = collection.get_package("com.anatawa12.library", prerelease);
        assert_installing_to_locked_only(&result, &library);

        assert_len!(result.package_changes(), 1);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);
    })
}

#[test]
fn resolve_unlocked_satisfies_all() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_file(
                "Packages/unlocked/package.json",
                r#"
            {
                "name": "com.anatawa12.package",
                "version": "1.0.0",
                "vpmDependencies": {
                    "com.anatawa12.library": "^1.0.0"
                }
            }
            "#,
            )
            // note:
            // the version of unlocked2 (com.anatawa12.library)
            // does not satisfy the requirement range of com.anatawa12.package
            .add_file(
                "Packages/unlocked2/package.json",
                r#"
            {
                "name": "com.anatawa12.library",
                "version": "2.0.0"
            }
            "#,
            )
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new().build();

        let result = project.resolve_request(&collection).await.unwrap();

        assert_len!(result.package_changes(), 0);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);
    })
}

#[test]
fn resolve_both_dependencies_of_unlocked_and_locked() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_file(
                "Packages/unlocked/package.json",
                r#"
            {
                "name": "com.anatawa12.package",
                "version": "1.0.0",
                "vpmDependencies": {
                    "com.anatawa12.library": "^1.0.0"
                }
            }
            "#,
            )
            .add_locked("com.anatawa12.library2", Version::new(1, 0, 0), &[])
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(PackageManifest::new(
                "com.anatawa12.library",
                Version::new(1, 0, 0),
            ))
            .add(PackageManifest::new(
                "com.anatawa12.library2",
                Version::new(1, 0, 0),
            ))
            .build();

        let result = project.resolve_request(&collection).await.unwrap();

        assert_len!(result.package_changes(), 2);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);

        let library = collection.get_package("com.anatawa12.library", Version::new(1, 0, 0));
        let library2 = collection.get_package("com.anatawa12.library2", Version::new(1, 0, 0));
        assert_installing_to_locked_only(&result, &library);
        assert_install_only(&result, &library2);
    })
}

#[test]
fn resolve_both_dependencies_of_unlocked_and_dependencies() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_file(
                "Packages/unlocked/package.json",
                r#"
            {
                "name": "com.anatawa12.package",
                "version": "1.0.0",
                "vpmDependencies": {
                    "com.anatawa12.library": "^1.0.0"
                }
            }
            "#,
            )
            .add_dependency_range("com.anatawa12.library2", "^1.0.0")
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(PackageManifest::new(
                "com.anatawa12.library",
                Version::new(1, 0, 0),
            ))
            .add(PackageManifest::new(
                "com.anatawa12.library2",
                Version::new(1, 0, 0),
            ))
            .build();

        let result = project.resolve_request(&collection).await.unwrap();

        assert_len!(result.package_changes(), 2);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);

        let library = collection.get_package("com.anatawa12.library", Version::new(1, 0, 0));
        let library2 = collection.get_package("com.anatawa12.library2", Version::new(1, 0, 0));
        assert_installing_to_locked_only(&result, &library);
        assert_installing_to_both(&result, &library2);
    })
}

// https://github.com/vrc-get/vrc-get/issues/2728
#[test]
fn dependencies_only_package_require_newer_version_than_locked_version() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_locked("com.anatawa12.locked", Version::new(1, 0, 0), &[])
            .add_dependency("com.anatawa12.dependency", Version::new(1, 1, 0))
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(PackageManifest::new(
                "com.anatawa12.locked",
                Version::new(1, 0, 0),
            ))
            .add(PackageManifest::new(
                "com.anatawa12.locked",
                Version::new(1, 1, 0),
            ))
            .add(
                PackageManifest::new("com.anatawa12.dependency", Version::new(1, 1, 0))
                    .add_vpm_dependency("com.anatawa12.locked", "^1.1.0"),
            )
            .build();

        let result = project.resolve_request(&collection).await.unwrap();

        assert_len!(result.package_changes(), 2);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);

        let library = collection.get_package("com.anatawa12.locked", Version::new(1, 1, 0));
        let library2 = collection.get_package("com.anatawa12.dependency", Version::new(1, 1, 0));
        assert_installing_to_locked_only(&result, &library);
        assert_installing_to_both(&result, &library2);
    })
}

#[test]
fn dependencies_only_package_is_replaced_for_another_dependencies_only_package() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency_range("com.anatawa12.library", "^1.0.0")
            .add_dependency("com.anatawa12.tool", Version::new(1, 0, 0))
            .build()
            .await
            .unwrap();

        // The collection for tests returns the first match so newer versions come first.
        let collection = PackageCollectionBuilder::new()
            .add(PackageManifest::new(
                "com.anatawa12.library",
                Version::new(1, 5, 0),
            ))
            .add(PackageManifest::new(
                "com.anatawa12.library",
                Version::new(1, 2, 0),
            ))
            .add(
                PackageManifest::new("com.anatawa12.tool", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.anatawa12.library", "<1.3.0"),
            )
            .build();

        let result = project.resolve_request(&collection).await.unwrap();

        // We select the library 1.5.0 at first since it's the newest, but nobody requested
        // the version. So we replace it with 1.2.0 since it satisfies both the project and the tool.
        let library = collection.get_package("com.anatawa12.library", Version::new(1, 2, 0));
        let tool = collection.get_package("com.anatawa12.tool", Version::new(1, 0, 0));
        assert_installing_to_both(&result, &library);
        assert_installing_to_both(&result, &tool);

        assert_len!(result.package_changes(), 2);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);
    })
}

#[test]
fn dependencies_only_package_require_older_version_than_locked_version() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_locked("com.anatawa12.locked", Version::new(1, 1, 0), &[])
            .add_dependency("com.anatawa12.dependency", Version::new(1, 1, 0))
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(PackageManifest::new(
                "com.anatawa12.locked",
                Version::new(1, 0, 0),
            ))
            .add(PackageManifest::new(
                "com.anatawa12.locked",
                Version::new(1, 1, 0),
            ))
            .add(
                PackageManifest::new("com.anatawa12.dependency", Version::new(1, 1, 0))
                    .add_vpm_dependency("com.anatawa12.locked", "1.0.x"),
            )
            .build();

        let result = project.resolve_request(&collection).await.unwrap();

        // the locked package must not be downgraded on resolve
        assert_len!(result.package_changes(), 2);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 1);

        let locked = collection.get_package("com.anatawa12.locked", Version::new(1, 1, 0));
        let dependency = collection.get_package("com.anatawa12.dependency", Version::new(1, 1, 0));
        assert_install_only(&result, &locked);
        assert_installing_to_both(&result, &dependency);

        let locked_conflict = result.conflicts().get("com.anatawa12.locked").unwrap();
        assert_eq!(
            locked_conflict.conflicting_packages(),
            &["com.anatawa12.dependency".into()]
        )
    })
}

// relates to https://github.com/vrc-get/vrc-get/issues/2728 but not exactly the what reported
#[test]
fn dependencies_of_unlocked_package_require_newer_version_than_locked_version() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_file(
                "Packages/unlocked/package.json",
                r#"
            {
                "name": "com.anatawa12.package",
                "version": "1.0.0",
                "vpmDependencies": {
                    "com.anatawa12.dependency": "^1.1.0"
                }
            }
            "#,
            )
            .add_locked("com.anatawa12.locked", Version::new(1, 0, 0), &[])
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(PackageManifest::new(
                "com.anatawa12.locked",
                Version::new(1, 0, 0),
            ))
            .add(PackageManifest::new(
                "com.anatawa12.locked",
                Version::new(1, 1, 0),
            ))
            .add(
                PackageManifest::new("com.anatawa12.dependency", Version::new(1, 1, 0))
                    .add_vpm_dependency("com.anatawa12.locked", "^1.1.0"),
            )
            .build();

        let result = project.resolve_request(&collection).await.unwrap();

        assert_len!(result.package_changes(), 2);
        assert_len!(result.remove_legacy_folders(), 0);
        assert_len!(result.remove_legacy_files(), 0);
        assert_len!(result.conflicts(), 0);

        let library = collection.get_package("com.anatawa12.locked", Version::new(1, 1, 0));
        let library2 = collection.get_package("com.anatawa12.dependency", Version::new(1, 1, 0));
        assert_installing_to_locked_only(&result, &library);
        assert_installing_to_locked_only(&result, &library2);
    })
}

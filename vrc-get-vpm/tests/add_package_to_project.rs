use common::*;
use std::collections::HashSet;
use std::io;
use std::path::Path;
use vrc_get_vpm::PackageManifest;
use vrc_get_vpm::io::IoTrait;
use vrc_get_vpm::unity_project::pending_project_changes::RemoveReason;
use vrc_get_vpm::unity_project::{AddPackageErr, AddPackageOperation};
use vrc_get_vpm::version::Version;

mod common;

// region basic operations

#[test]
fn add_to_dependencies() {
    block_on(async {
        let project = VirtualProjectBuilder::new().build().await.unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.vrchat.avatars", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.0.0"),
            )
            .add(PackageManifest::new(
                "com.vrchat.base",
                Version::new(1, 0, 0),
            ))
            .build();

        let avatars_package = collection.get_package("com.vrchat.avatars", Version::new(1, 0, 0));
        let base_package = collection.get_package("com.vrchat.base", Version::new(1, 0, 0));

        let result = project
            .add_package_request(
                &collection,
                &[avatars_package],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 2);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

        assert_installing_to_both(&result, &avatars_package);
        assert_installing_to_locked_only(&result, &base_package);
    })
}

#[test]
fn install_already_installed_in_locked_to_locked() {
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

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.vrchat.avatars", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.0.0"),
            )
            .add(PackageManifest::new(
                "com.vrchat.base",
                Version::new(1, 0, 0),
            ))
            .build();

        let avatars_package = collection.get_package("com.vrchat.avatars", Version::new(1, 0, 0));

        let result = project
            .add_package_request(
                &collection,
                &[avatars_package],
                AddPackageOperation::UpgradeLocked,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 0);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);
    })
}

#[test]
fn install_already_installed_in_locked_to_dependencies() {
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

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.vrchat.avatars", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.0.0"),
            )
            .add(PackageManifest::new(
                "com.vrchat.base",
                Version::new(1, 0, 0),
            ))
            .build();

        let base_package = collection.get_package("com.vrchat.base", Version::new(1, 0, 0));

        let result = project
            .add_package_request(
                &collection,
                &[base_package],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 1);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

        assert_installing_to_dependencies_only(&result, "com.vrchat.base", Version::new(1, 0, 0));
    })
}

#[test]
fn install_already_installed_in_dependencies_to_dependencies() {
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

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.vrchat.avatars", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.0.0"),
            )
            .add(PackageManifest::new(
                "com.vrchat.base",
                Version::new(1, 0, 0),
            ))
            .build();

        let avatars_package = collection.get_package("com.vrchat.avatars", Version::new(1, 0, 0));

        let result = project
            .add_package_request(
                &collection,
                &[avatars_package],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 0);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);
    })
}

#[test]
fn upgrading_unused_packages() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_locked(
                "com.vrchat.avatars",
                Version::new(1, 0, 0),
                &[("com.vrchat.base", "1.0.0")],
            )
            .add_locked("com.vrchat.base", Version::new(1, 0, 0), &[])
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.vrchat.avatars", Version::new(1, 1, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.1.0"),
            )
            .add(PackageManifest::new(
                "com.vrchat.base",
                Version::new(1, 1, 0),
            ))
            .build();

        let avatars_package = collection.get_package("com.vrchat.avatars", Version::new(1, 1, 0));
        let base_package = collection.get_package("com.vrchat.base", Version::new(1, 1, 0));

        let result = project
            .add_package_request(
                &collection,
                &[avatars_package],
                AddPackageOperation::UpgradeLocked,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 2);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

        assert_installing_to_locked_only(&result, &avatars_package);
        assert_installing_to_locked_only(&result, &base_package);
    })
}

// endregion

// region remove unused

#[test]
fn transitive_unused_remove_with_upgrade() {
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
            .add(PackageManifest::new(
                "com.anatawa12.package",
                Version::new(1, 1, 0),
            ))
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 1, 0));

        let result = project
            .add_package_request(
                &collection,
                &[package],
                AddPackageOperation::UpgradeLocked,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 2);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

        assert_installing_to_locked_only(&result, &package);
        assert_removed(&result, "com.anatawa12.library", RemoveReason::Unused);
    })
}

#[test]
fn do_not_remove_transitively_when_untouched() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency("com.anatawa12.package", Version::new(1, 0, 0))
            .add_locked(
                "com.anatawa12.package",
                Version::new(1, 0, 0),
                &[("com.anatawa12.library", "1.0.0")],
            )
            .add_locked("com.anatawa12.library", Version::new(1, 0, 0), &[])
            .add_locked(
                "com.anatawa12.untouched_library",
                Version::new(1, 0, 0),
                &[],
            )
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(PackageManifest::new(
                "com.anatawa12.package",
                Version::new(1, 1, 0),
            ))
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 1, 0));

        let result = project
            .add_package_request(
                &collection,
                &[package],
                AddPackageOperation::UpgradeLocked,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 2);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

        assert_installing_to_locked_only(&result, &package);
        assert_removed(&result, "com.anatawa12.library", RemoveReason::Unused);
    })
}

#[test]
fn remove_legacy_package_when_install() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency("com.anatawa12.legacy-package", Version::new(1, 0, 0))
            .add_locked("com.anatawa12.legacy-package", Version::new(1, 0, 0), &[])
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.anatawa12.package", Version::new(1, 1, 0))
                    .add_legacy_package("com.anatawa12.legacy-package"),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 1, 0));

        let result = project
            .add_package_request(
                &collection,
                &[package],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 2);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

        assert_installing_to_both(&result, &package);
        assert_removed(
            &result,
            "com.anatawa12.legacy-package",
            RemoveReason::Legacy,
        );
    })
}

#[test]
fn remove_legacy_package_when_upgrade() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency("com.anatawa12.package", Version::new(1, 0, 0))
            .add_locked("com.anatawa12.legacy-package", Version::new(1, 0, 0), &[])
            .add_locked("com.anatawa12.package", Version::new(1, 0, 0), &[])
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.anatawa12.package", Version::new(1, 1, 0))
                    .add_legacy_package("com.anatawa12.legacy-package"),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 1, 0));

        let result = project
            .add_package_request(
                &collection,
                &[package],
                AddPackageOperation::UpgradeLocked,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 2);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

        assert_installing_to_locked_only(&result, &package);
        assert_removed(
            &result,
            "com.anatawa12.legacy-package",
            RemoveReason::Legacy,
        );
    })
}

#[test]
fn remove_referenced_legacy_package_when_install() {
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
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.anatawa12.package", Version::new(1, 1, 0))
                    .add_legacy_package("com.anatawa12.legacy-package"),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 1, 0));

        let result = project
            .add_package_request(
                &collection,
                &[package],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 2);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

        assert_installing_to_both(&result, &package);
        assert_removed(
            &result,
            "com.anatawa12.legacy-package",
            RemoveReason::Legacy,
        );
    })
}

// endregion

//region legacy assets

#[test]
fn legacy_assets_by_path() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dir("Assets/LegacyFolder")
            .add_dir("Packages/legacy.package") // VRCSDK Worlds uses legacy dir for removing package
            .add_file("Assets/LegacyAsset.cs", "// empty file")
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.anatawa12.package", Version::new(1, 0, 0))
                    .add_legacy_folder("Assets\\LegacyFolder", "")
                    .add_legacy_folder("Assets\\NotExists", "")
                    .add_legacy_folder("Packages\\legacy.package", "")
                    .add_legacy_file("Assets\\LegacyAsset.cs", ""),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 0, 0));

        let result = project
            .add_package_request(
                &collection,
                &[package],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 1);
        assert_eq!(result.conflicts().len(), 0);

        assert_eq!(
            result
                .remove_legacy_folders()
                .iter()
                .collect::<HashSet<_>>(),
            [
                (
                    Path::new("Assets/LegacyFolder").into(),
                    "com.anatawa12.package",
                ),
                (
                    Path::new("Packages/legacy.package").into(),
                    "com.anatawa12.package",
                ),
            ]
            .iter()
            .collect::<HashSet<_>>()
        );

        assert_eq!(
            result.remove_legacy_files().iter().collect::<HashSet<_>>(),
            [(
                Path::new("Assets/LegacyAsset.cs").into(),
                "com.anatawa12.package",
            )]
            .iter()
            .collect::<HashSet<_>>()
        );
    })
}

#[test]
fn legacy_assets_by_guid() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dir("Assets/MovedLegacyFolder")
            .add_file(
                "Assets/MovedLegacyFolder.meta",
                "guid: 1c54b633da4d4d2abc01c6dedae67e09",
            )
            .add_file("Assets/MovedLegacyAsset.cs", "// empty file")
            .add_file(
                "Assets/MovedLegacyAsset.cs.meta",
                "guid: ca06b0788d62432083b3577cc2346126",
            )
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.anatawa12.package", Version::new(1, 0, 0))
                    .add_legacy_folder("Assets\\LegacyFolder", "1c54b633da4d4d2abc01c6dedae67e09")
                    .add_legacy_folder("Assets\\NotExists", "62a9615044174c818622c19d0181d036")
                    .add_legacy_file("Assets\\LegacyAsset.cs", "ca06b0788d62432083b3577cc2346126"),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 0, 0));

        let result = project
            .add_package_request(
                &collection,
                &[package],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 1);
        assert_eq!(result.conflicts().len(), 0);

        assert_eq!(
            result
                .remove_legacy_folders()
                .iter()
                .collect::<HashSet<_>>(),
            [(
                Path::new("Assets/MovedLegacyFolder").into(),
                "com.anatawa12.package",
            )]
            .iter()
            .collect::<HashSet<_>>()
        );

        assert_eq!(
            result.remove_legacy_files().iter().collect::<HashSet<_>>(),
            [(
                Path::new("Assets/MovedLegacyAsset.cs").into(),
                "com.anatawa12.package",
            )]
            .iter()
            .collect::<HashSet<_>>()
        );
    })
}

#[test]
fn deny_remove_files_not_in_assets_or_packages() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dir("Assets1/LegacyFolder")
            .add_dir("Packages1/legacy.package") // VRCSDK Worlds uses legacy dir for removing package
            .add_file("Assets1/LegacyAsset.cs", "// empty file")
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.anatawa12.package", Version::new(1, 0, 0))
                    .add_legacy_folder("Assets1\\LegacyFolder", "")
                    .add_legacy_folder("Assets1\\NotExists", "")
                    .add_legacy_folder("Packages1\\legacy.package", "")
                    .add_legacy_file("Assets1\\LegacyAsset.cs", ""),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 0, 0));

        let result = project
            .add_package_request(
                &collection,
                &[package],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 1);
        assert_eq!(result.conflicts().len(), 0);

        assert_eq!(result.remove_legacy_folders(), &[]);
        assert_eq!(result.remove_legacy_files(), &[]);
    })
}

#[test]
fn deny_remove_parent_folders() {
    block_on(async {
        let project = VirtualProjectBuilder::new().build().await.unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.anatawa12.package", Version::new(1, 0, 0))
                    .add_legacy_folder("..", "")
                    .add_legacy_folder("Assets/..", "")
                    .add_legacy_folder("", ""),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 0, 0));

        let result = project
            .add_package_request(
                &collection,
                &[package],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 1);
        assert_eq!(result.conflicts().len(), 0);

        assert_eq!(result.remove_legacy_folders(), &[]);
        assert_eq!(result.remove_legacy_files(), &[]);
    })
}

#[test]
fn deny_absolute_legacy_assets() {
    block_on(async {
        let project = VirtualProjectBuilder::new().build().await.unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.anatawa12.package", Version::new(1, 0, 0))
                    .add_legacy_folder("/", ""),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 0, 0));

        let result = project
            .add_package_request(
                &collection,
                &[package],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 1);
        assert_eq!(result.conflicts().len(), 0);

        assert_eq!(result.remove_legacy_folders(), &[]);
        assert_eq!(result.remove_legacy_files(), &[]);
    })
}

#[test]
fn do_remove_legacy_files_that_guid_mismatches() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dir("Assets/LegacyGuidMismatchFolder")
            .add_file(
                "Assets/LegacyGuidMismatchFolder.meta",
                "guid: 75ead46ee7514680afd1ed9008423371",
            )
            .add_dir("Assets/LegacyGuidMatchFolder")
            .add_file(
                "Assets/LegacyGuidMatchFolder.meta",
                "guid: cbac8a2877e64d75af9b3b61fe946b40",
            )
            .add_file("Assets/LegacyGuidMismatchAsset.cs", "// empty file")
            .add_file(
                "Assets/LegacyGuidMismatchAsset.cs.meta",
                "guid: 85cc942c4a8a487ebe7df9937d15bdf8",
            )
            .add_file("Assets/LegacyGuidMatchAsset.cs", "// empty file")
            .add_file(
                "Assets/LegacyGuidMatchAsset.cs.meta",
                "guid: 85cc942c4a8a487ebe7df9937d15bdf8",
            )
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.anatawa12.package", Version::new(1, 0, 0))
                    .add_legacy_folder(
                        "Assets\\LegacyGuidMismatchFolder",
                        "39bfddf4f3ac48c6845997a81fc5262c",
                    )
                    .add_legacy_folder(
                        "Assets\\LegacyGuidMatchFolder",
                        "cbac8a2877e64d75af9b3b61fe946b40",
                    )
                    .add_legacy_file(
                        "Assets\\LegacyGuidMismatchAsset.cs",
                        "417a8085479b433792a67b2dfefb1982",
                    )
                    .add_legacy_file(
                        "Assets\\LegacyGuidMatchAsset.cs",
                        "85cc942c4a8a487ebe7df9937d15bdf8",
                    ),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 0, 0));

        let result = project
            .add_package_request(
                &collection,
                &[package],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 1);
        assert_eq!(result.conflicts().len(), 0);

        assert_eq!(
            result
                .remove_legacy_folders()
                .iter()
                .collect::<HashSet<_>>(),
            [
                (
                    Path::new("Assets/LegacyGuidMatchFolder").into(),
                    "com.anatawa12.package",
                ),
                (
                    Path::new("Assets/LegacyGuidMismatchFolder").into(),
                    "com.anatawa12.package",
                ),
            ]
            .iter()
            .collect::<HashSet<_>>()
        );

        assert_eq!(
            result.remove_legacy_files().iter().collect::<HashSet<_>>(),
            [
                (
                    Path::new("Assets/LegacyGuidMatchAsset.cs").into(),
                    "com.anatawa12.package",
                ),
                (
                    Path::new("Assets/LegacyGuidMismatchAsset.cs").into(),
                    "com.anatawa12.package",
                ),
            ]
            .iter()
            .collect::<HashSet<_>>()
        );
    })
}

#[test]
fn do_not_remove_udonsharp_folder_if_guid_mismatch() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dir("Assets/UdonSharp")
            .add_file(
                "Assets/UdonSharp.meta",
                "guid: e2095dec983b4d9481723c263fd5b6c0",
            )
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.vrchat.worlds", Version::new(1, 0, 0))
                    .add_legacy_folder("Assets\\UdonSharp", "b031f928e5c709b4887f6513084aaa51"),
            )
            .build();

        let package = collection.get_package("com.vrchat.worlds", Version::new(1, 0, 0));

        let result = project
            .add_package_request(
                &collection,
                &[package],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 1);
        assert_eq!(result.conflicts().len(), 0);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
    })
}

//endregion

// region errors

#[test]
fn not_found_err() {
    block_on(async {
        let project = VirtualProjectBuilder::new().build().await.unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.vrchat.avatars", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.0.0"),
            )
            .build();

        let avatars_package = collection.get_package("com.vrchat.avatars", Version::new(1, 0, 0));

        let err = project
            .add_package_request(
                &collection,
                &[avatars_package],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .expect_err("should fail");

        match &err {
            AddPackageErr::DependenciesNotFound { dependencies } => {
                assert_eq!(dependencies.len(), 1);
                assert_eq!(dependencies[0].0.as_ref(), "com.vrchat.base");
            }
            _ => panic!("unexpected error: {err:?}"),
        }
    })
}

#[test]
fn updating_non_locked_package_should_cause_error() {
    block_on(async {
        let project = VirtualProjectBuilder::new().build().await.unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(PackageManifest::new(
                "com.vrchat.avatars",
                Version::new(1, 0, 0),
            ))
            .build();

        let avatars_package = collection.get_package("com.vrchat.avatars", Version::new(1, 0, 0));

        let err = project
            .add_package_request(
                &collection,
                &[avatars_package],
                AddPackageOperation::UpgradeLocked,
                false,
            )
            .await
            .expect_err("should fail");

        match &err {
            AddPackageErr::UpgradingNonLockedPackage { package_name } => {
                assert_eq!(package_name.as_ref(), "com.vrchat.avatars");
            }
            _ => panic!("unexpected error: {err:?}"),
        }
    })
}

#[test]
fn downgrade_basic() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_locked("com.vrchat.base", Version::new(1, 1, 0), &[])
            .add_dependency("com.vrchat.base", Version::new(1, 0, 0))
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(PackageManifest::new(
                "com.vrchat.base",
                Version::new(1, 0, 0),
            ))
            .build();

        let base_package = collection.get_package("com.vrchat.base", Version::new(1, 0, 0));

        let result = project
            .add_package_request(
                &collection,
                &[base_package],
                AddPackageOperation::Downgrade,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 1);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

        assert_installing_to_locked_only(&result, &base_package);
    })
}

#[test]
fn downgrade_dependencies() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_locked("com.vrchat.base", Version::new(1, 1, 0), &[])
            .add_dependency("com.vrchat.base", Version::new(1, 1, 0))
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(PackageManifest::new(
                "com.vrchat.base",
                Version::new(1, 0, 0),
            ))
            .build();

        let base_package = collection.get_package("com.vrchat.base", Version::new(1, 0, 0));

        let result = project
            .add_package_request(
                &collection,
                &[base_package],
                AddPackageOperation::Downgrade,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 1);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

        assert_installing_to_both(&result, &base_package);
    })
}

// endregion

// region conflicts

#[test]
fn conflict_requirements_of_installed_and_installing() {
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

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.vrchat.avatars", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.0.0"),
            )
            .add(PackageManifest::new(
                "com.vrchat.base",
                Version::new(1, 0, 0),
            ))
            .add(PackageManifest::new(
                "com.vrchat.base",
                Version::new(1, 1, 0),
            ))
            .add(
                PackageManifest::new("com.anatawa12.tool", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "^1.1.0"),
            )
            .build();

        let tool = collection.get_package("com.anatawa12.tool", Version::new(1, 0, 0));
        let base_1_1_0 = collection.get_package("com.vrchat.base", Version::new(1, 1, 0));

        let resolve = project
            .add_package_request(
                &collection,
                &[tool],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(resolve.package_changes().len(), 2);
        assert_eq!(resolve.remove_legacy_folders().len(), 0);
        assert_eq!(resolve.remove_legacy_files().len(), 0);
        assert_eq!(resolve.conflicts().len(), 1);

        assert_installing_to_both(&resolve, &tool);
        assert_installing_to_locked_only(&resolve, &base_1_1_0);

        let base_conflict = resolve.conflicts().get("com.vrchat.base").unwrap();
        assert_eq!(
            base_conflict.conflicting_packages(),
            &["com.vrchat.avatars".into()]
        )
    })
}

#[test]
fn conflict_already_conflicted_and_no_new_conflict() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency("com.vrchat.avatars", Version::new(1, 0, 0))
            .add_locked(
                "com.vrchat.avatars",
                Version::new(1, 0, 0),
                &[("com.vrchat.base", "1.0.0")],
            )
            .add_locked("com.vrchat.base", Version::new(1, 1, 0), &[])
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.vrchat.avatars", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.0.0"),
            )
            .add(PackageManifest::new(
                "com.vrchat.base",
                Version::new(1, 0, 0),
            ))
            .add(PackageManifest::new(
                "com.vrchat.base",
                Version::new(1, 1, 0),
            ))
            .add(
                PackageManifest::new("com.anatawa12.tool", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "^1.1.0"),
            )
            .build();

        let tool = collection.get_package("com.anatawa12.tool", Version::new(1, 0, 0));

        let resolve = project
            .add_package_request(
                &collection,
                &[tool],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(resolve.package_changes().len(), 1);
        assert_eq!(resolve.remove_legacy_folders().len(), 0);
        assert_eq!(resolve.remove_legacy_files().len(), 0);
        assert_eq!(resolve.conflicts().len(), 0);

        assert_installing_to_both(&resolve, &tool);
    })
}

#[test]
fn conflict_requirements_of_installed_and_installing_related_to_dependencies() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency_range("com.vrchat.avatars", "~3.5.x")
            .add_locked(
                "com.vrchat.avatars",
                Version::new(3, 4, 2),
                &[("com.vrchat.base", "3.4.2")],
            )
            .add_locked("com.vrchat.base", Version::new(3, 4, 2), &[])
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.vrchat.base", Version::new(3, 4, 2))
                    .add_vpm_dependency("com.vrchat.avatars", "3.4.2"),
            )
            .add(PackageManifest::new(
                "com.vrchat.base",
                Version::new(3, 4, 2),
            ))
            .add(
                PackageManifest::new("com.anatawa12.tool", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.avatars", "^3.3.0"),
            )
            .build();

        let tool = collection.get_package("com.anatawa12.tool", Version::new(1, 0, 0));

        let resolve = project
            .add_package_request(
                &collection,
                &[tool],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(resolve.package_changes().len(), 1);
        assert_eq!(resolve.remove_legacy_folders().len(), 0);
        assert_eq!(resolve.remove_legacy_files().len(), 0);
        assert_eq!(resolve.conflicts().len(), 0);

        assert_installing_to_both(&resolve, &tool);
    })
}

// endregion

// region rollback on error

#[test]
fn no_temp_folder_after_add() {
    block_on(async {
        let mut project = VirtualProjectBuilder::new()
            .add_dependency_range("com.vrchat.avatars", "~3.5.x")
            .add_locked(
                "com.vrchat.avatars",
                Version::new(3, 4, 2),
                &[("com.vrchat.base", "3.4.2")],
            )
            .add_locked("com.vrchat.base", Version::new(3, 4, 2), &[])
            .add_file(
                "Packages/com.vrchat.avatars/package.json",
                r#"{"name":"com.vrchat.avatars","version":"3.4.2"}"#,
            )
            .add_file("Packages/com.vrchat.avatars/content.txt", "text")
            .build()
            .await
            .unwrap();

        let env = VirtualInstaller::new();

        let resolve = project
            .remove_request(&["com.vrchat.avatars"])
            .await
            .unwrap();

        project.apply_pending_changes(&env, resolve).await.unwrap();

        assert_eq!(
            project
                .io()
                .metadata("Temp".as_ref())
                .await
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            project
                .io()
                .metadata("Packages/com.vrchat.avatars".as_ref())
                .await
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotFound
        );
    })
}

#[test]
#[ignore = "No suitable way to lock a file"]
fn locked_in_package_folder() {
    block_on(async {
        let mut project = VirtualProjectBuilder::new()
            .add_dependency_range("com.vrchat.avatars", "~3.5.x")
            .add_locked(
                "com.vrchat.avatars",
                Version::new(3, 4, 2),
                &[("com.vrchat.base", "3.4.2")],
            )
            .add_locked("com.vrchat.base", Version::new(3, 4, 2), &[])
            .add_file(
                "Packages/com.vrchat.avatars/package.json",
                r#"{"name":"com.vrchat.avatars","version":"3.4.2"}"#,
            )
            .add_file("Packages/com.vrchat.avatars/content.txt", "text")
            .build()
            .await
            .unwrap();

        //project.io().lock("Packages/com.vrchat.avatars/content.txt".as_ref()).await.unwrap();

        let env = VirtualInstaller::new();

        let resolve = project
            .remove_request(&["com.vrchat.avatars"])
            .await
            .unwrap();

        project.apply_pending_changes(&env, resolve).await.unwrap();

        assert_eq!(
            project
                .io()
                .metadata("Packages/com.vrchat.avatars".as_ref())
                .await
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotFound
        );
        project.io().metadata("Temp".as_ref()).await.unwrap();
        project
            .io()
            .metadata("Temp/vrc-get".as_ref())
            .await
            .unwrap();
    })
}

// endregion

// region unlocked

#[test]
fn install_depends_on_unlocked() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_file(
                "Packages/base/package.json",
                r#"{"name":"com.vrchat.base","version":"1.0.0"}"#,
            )
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.vrchat.avatars", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.0.0"),
            )
            .build();

        let avatars_package = collection.get_package("com.vrchat.avatars", Version::new(1, 0, 0));

        let result = project
            .add_package_request(
                &collection,
                &[avatars_package],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 1);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

        assert_installing_to_both(&result, &avatars_package);
    })
}

#[test]
fn install_conflict_with_unlocked() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_file(
                "Packages/base/package.json",
                r#"{"name":"com.vrchat.base","version":"1.0.0"}"#,
            )
            .build()
            .await
            .unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageManifest::new("com.vrchat.avatars", Version::new(1, 1, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.1.0"),
            )
            .build();

        let avatars_package = collection.get_package("com.vrchat.avatars", Version::new(1, 1, 0));

        let result = project
            .add_package_request(
                &collection,
                &[avatars_package],
                AddPackageOperation::InstallToDependencies,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 1);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 1);

        assert_installing_to_both(&result, &avatars_package);

        let base_conflict = result.conflicts().get("com.vrchat.base").unwrap();
        assert_eq!(
            base_conflict.conflicting_packages(),
            &["com.vrchat.avatars".into()]
        )
    })
}

// endregion

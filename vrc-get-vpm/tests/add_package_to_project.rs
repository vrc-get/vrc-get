use common::*;
use futures::executor::block_on;
use std::collections::HashSet;
use std::path::Path;
use vrc_get_vpm::unity_project::pending_project_changes::RemoveReason;
use vrc_get_vpm::unity_project::AddPackageErr;
use vrc_get_vpm::version::Version;
use vrc_get_vpm::PackageJson;

mod common;

// region basic operations

#[test]
fn add_to_dependencies() {
    block_on(async {
        let project = VirtualProjectBuilder::new().build().await.unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(
                PackageJson::new("com.vrchat.avatars", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.0.0"),
            )
            .add(PackageJson::new("com.vrchat.base", Version::new(1, 0, 0)))
            .build();

        let avatars_package = collection.get_package("com.vrchat.avatars", Version::new(1, 0, 0));
        let base_package = collection.get_package("com.vrchat.base", Version::new(1, 0, 0));

        let result = project
            .add_package_request(&collection, vec![avatars_package], true, false)
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
                PackageJson::new("com.vrchat.avatars", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.0.0"),
            )
            .add(PackageJson::new("com.vrchat.base", Version::new(1, 0, 0)))
            .build();

        let avatars_package = collection.get_package("com.vrchat.avatars", Version::new(1, 0, 0));

        let result = project
            .add_package_request(&collection, vec![avatars_package], false, false)
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
                PackageJson::new("com.vrchat.avatars", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.0.0"),
            )
            .add(PackageJson::new("com.vrchat.base", Version::new(1, 0, 0)))
            .build();

        let base_package = collection.get_package("com.vrchat.base", Version::new(1, 0, 0));

        let result = project
            .add_package_request(&collection, vec![base_package], true, false)
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
                PackageJson::new("com.vrchat.avatars", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.0.0"),
            )
            .add(PackageJson::new("com.vrchat.base", Version::new(1, 0, 0)))
            .build();

        let avatars_package = collection.get_package("com.vrchat.avatars", Version::new(1, 0, 0));

        let result = project
            .add_package_request(&collection, vec![avatars_package], true, false)
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 0);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);
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
            .add(PackageJson::new(
                "com.anatawa12.package",
                Version::new(1, 1, 0),
            ))
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 1, 0));

        let result = project
            .add_package_request(&collection, vec![package], false, false)
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
            .add(PackageJson::new(
                "com.anatawa12.package",
                Version::new(1, 1, 0),
            ))
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 1, 0));

        let result = project
            .add_package_request(&collection, vec![package], false, false)
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
                PackageJson::new("com.anatawa12.package", Version::new(1, 1, 0))
                    .add_legacy_package("com.anatawa12.legacy-package"),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 1, 0));

        let result = project
            .add_package_request(&collection, vec![package], true, false)
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
                PackageJson::new("com.anatawa12.package", Version::new(1, 1, 0))
                    .add_legacy_package("com.anatawa12.legacy-package"),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 1, 0));

        let result = project
            .add_package_request(&collection, vec![package], false, false)
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
                PackageJson::new("com.anatawa12.package", Version::new(1, 1, 0))
                    .add_legacy_package("com.anatawa12.legacy-package"),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 1, 0));

        let result = project
            .add_package_request(&collection, vec![package], true, false)
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
                PackageJson::new("com.anatawa12.package", Version::new(1, 0, 0))
                    .add_legacy_folder("Assets\\LegacyFolder", "")
                    .add_legacy_folder("Assets\\NotExists", "")
                    .add_legacy_folder("Packages\\legacy.package", "")
                    .add_legacy_file("Assets\\LegacyAsset.cs", ""),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 0, 0));

        let result = project
            .add_package_request(&collection, vec![package], true, false)
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
                Path::new("Assets/LegacyFolder").into(),
                Path::new("Packages/legacy.package").into()
            ]
            .iter()
            .collect::<HashSet<_>>()
        );

        assert_eq!(
            result.remove_legacy_files().iter().collect::<HashSet<_>>(),
            [Path::new("Assets/LegacyAsset.cs").into()]
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
                PackageJson::new("com.anatawa12.package", Version::new(1, 0, 0))
                    .add_legacy_folder("Assets\\LegacyFolder", "1c54b633da4d4d2abc01c6dedae67e09")
                    .add_legacy_folder("Assets\\NotExists", "62a9615044174c818622c19d0181d036")
                    .add_legacy_file("Assets\\LegacyAsset.cs", "ca06b0788d62432083b3577cc2346126"),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 0, 0));

        let result = project
            .add_package_request(&collection, vec![package], true, false)
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 1);
        assert_eq!(result.conflicts().len(), 0);

        assert_eq!(
            result
                .remove_legacy_folders()
                .iter()
                .collect::<HashSet<_>>(),
            [Path::new("Assets/MovedLegacyFolder").into(),]
                .iter()
                .collect::<HashSet<_>>()
        );

        assert_eq!(
            result.remove_legacy_files().iter().collect::<HashSet<_>>(),
            [Path::new("Assets/MovedLegacyAsset.cs").into()]
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
                PackageJson::new("com.anatawa12.package", Version::new(1, 0, 0))
                    .add_legacy_folder("Assets1\\LegacyFolder", "")
                    .add_legacy_folder("Assets1\\NotExists", "")
                    .add_legacy_folder("Packages1\\legacy.package", "")
                    .add_legacy_file("Assets1\\LegacyAsset.cs", ""),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 0, 0));

        let result = project
            .add_package_request(&collection, vec![package], true, false)
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
                PackageJson::new("com.anatawa12.package", Version::new(1, 0, 0))
                    .add_legacy_folder("..", "")
                    .add_legacy_folder("Assets/..", "")
                    .add_legacy_folder("", ""),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 0, 0));

        let result = project
            .add_package_request(&collection, vec![package], true, false)
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
                PackageJson::new("com.anatawa12.package", Version::new(1, 0, 0))
                    .add_legacy_folder("/", ""),
            )
            .build();

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 0, 0));

        let result = project
            .add_package_request(&collection, vec![package], true, false)
            .await
            .unwrap();

        assert_eq!(result.package_changes().len(), 1);
        assert_eq!(result.conflicts().len(), 0);

        assert_eq!(result.remove_legacy_folders(), &[]);
        assert_eq!(result.remove_legacy_files(), &[]);
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
                PackageJson::new("com.vrchat.avatars", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.0.0"),
            )
            .build();

        let avatars_package = collection.get_package("com.vrchat.avatars", Version::new(1, 0, 0));

        let err = project
            .add_package_request(&collection, vec![avatars_package], true, false)
            .await
            .expect_err("should fail");

        match &err {
            AddPackageErr::DependencyNotFound { dependency_name } => {
                assert_eq!(dependency_name.as_ref(), "com.vrchat.base");
            }
            _ => panic!("unexpected error: {:?}", err),
        }
    })
}

#[test]
fn updating_non_locked_package_should_cause_error() {
    block_on(async {
        let project = VirtualProjectBuilder::new().build().await.unwrap();

        let collection = PackageCollectionBuilder::new()
            .add(PackageJson::new(
                "com.vrchat.avatars",
                Version::new(1, 0, 0),
            ))
            .build();

        let avatars_package = collection.get_package("com.vrchat.avatars", Version::new(1, 0, 0));

        let err = project
            .add_package_request(&collection, vec![avatars_package], false, false)
            .await
            .expect_err("should fail");

        match &err {
            AddPackageErr::UpdateingNonLockedPackage { package_name } => {
                assert_eq!(package_name.as_ref(), "com.vrchat.avatars");
            }
            _ => panic!("unexpected error: {:?}", err),
        }
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
                PackageJson::new("com.vrchat.avatars", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.0.0"),
            )
            .add(PackageJson::new("com.vrchat.base", Version::new(1, 0, 0)))
            .add(PackageJson::new("com.vrchat.base", Version::new(1, 1, 0)))
            .add(
                PackageJson::new("com.anatawa12.tool", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "^1.1.0"),
            )
            .build();

        let tool = collection.get_package("com.anatawa12.tool", Version::new(1, 0, 0));
        let base_1_1_0 = collection.get_package("com.vrchat.base", Version::new(1, 1, 0));

        let resolve = project
            .add_package_request(&collection, vec![tool], true, false)
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
                PackageJson::new("com.vrchat.avatars", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "1.0.0"),
            )
            .add(PackageJson::new("com.vrchat.base", Version::new(1, 0, 0)))
            .add(PackageJson::new("com.vrchat.base", Version::new(1, 1, 0)))
            .add(
                PackageJson::new("com.anatawa12.tool", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.base", "^1.1.0"),
            )
            .build();

        let tool = collection.get_package("com.anatawa12.tool", Version::new(1, 0, 0));

        let resolve = project
            .add_package_request(&collection, vec![tool], true, false)
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
                PackageJson::new("com.vrchat.base", Version::new(3, 4, 2))
                    .add_vpm_dependency("com.vrchat.avatars", "3.4.2"),
            )
            .add(PackageJson::new("com.vrchat.base", Version::new(3, 4, 2)))
            .add(
                PackageJson::new("com.anatawa12.tool", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.vrchat.avatars", "^3.3.0"),
            )
            .build();

        let tool = collection.get_package("com.anatawa12.tool", Version::new(1, 0, 0));

        let resolve = project
            .add_package_request(&collection, vec![tool], true, false)
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

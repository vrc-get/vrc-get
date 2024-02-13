use common::*;
use futures::executor::block_on;
use vrc_get_vpm::unity_project::pending_project_changes::RemoveReason;
use vrc_get_vpm::version::Version;
use vrc_get_vpm::PackageJson;

mod common;

#[test]
fn add_to_locked_only() {
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
            .add_package_request(&collection, vec![avatars_package], false, false)
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

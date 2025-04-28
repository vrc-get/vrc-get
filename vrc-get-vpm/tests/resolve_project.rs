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

        assert_eq!(result.package_changes().len(), 2);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

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

        assert_eq!(result.package_changes().len(), 2);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

        let package = collection.get_package("com.anatawa12.package", Version::new(1, 0, 0));
        let library = collection.get_package("com.anatawa12.library", Version::new(1, 0, 0));
        assert_installing_to_both(&result, &package);
        assert_installing_to_locked_only(&result, &library);
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

        assert_eq!(result.package_changes().len(), 1);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

        let library = collection.get_package("com.anatawa12.library", Version::new(1, 0, 0));
        assert_installing_to_locked_only(&result, &library);
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

        assert_eq!(result.package_changes().len(), 0);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);
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

        assert_eq!(result.package_changes().len(), 2);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

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

        assert_eq!(result.package_changes().len(), 2);
        assert_eq!(result.remove_legacy_folders().len(), 0);
        assert_eq!(result.remove_legacy_files().len(), 0);
        assert_eq!(result.conflicts().len(), 0);

        let library = collection.get_package("com.anatawa12.library", Version::new(1, 0, 0));
        let library2 = collection.get_package("com.anatawa12.library2", Version::new(1, 0, 0));
        assert_installing_to_locked_only(&result, &library);
        assert_installing_to_both(&result, &library2);
    })
}

use crate::common::*;
use futures::executor::block_on;
use vrc_get_vpm::version::Version;
use vrc_get_vpm::PackageJson;

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
                PackageJson::new("com.anatawa12.package", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.anatawa12.library", "1.0.0"),
            )
            .add(PackageJson::new(
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
                PackageJson::new("com.anatawa12.package", Version::new(1, 0, 0))
                    .add_vpm_dependency("com.anatawa12.library", "1.0.0"),
            )
            .add(PackageJson::new(
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

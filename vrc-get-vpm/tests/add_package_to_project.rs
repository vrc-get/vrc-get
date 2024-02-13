use crate::common::{PackageCollectionBuilder, VirtualProjectBuilder};
use futures::executor::block_on;
use vrc_get_vpm::unity_project::pending_project_changes::RemoveReason;
use vrc_get_vpm::version::{DependencyRange, Version};
use vrc_get_vpm::PackageJson;

mod common;

#[test]
fn add_to_locked_only() {
    block_on(async {
        let project = VirtualProjectBuilder::new().build().await.unwrap();

        // create package collection
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

        let avatars_change = result.package_changes().get("com.vrchat.avatars").unwrap();
        let avatars_change = avatars_change
            .as_install()
            .expect("avatars is not installing");
        assert!(avatars_change.is_adding_to_locked());
        assert!(avatars_change.to_dependencies().is_none());
        let avatars_pkg = avatars_change.install_package().expect("no package");
        assert_eq!(avatars_pkg.name(), "com.vrchat.avatars");
        assert_eq!(avatars_pkg.version(), &Version::new(1, 0, 0));
        assert_eq!(
            avatars_pkg.package_json() as *const _,
            avatars_package.package_json() as *const _
        );

        let base_change = result.package_changes().get("com.vrchat.base").unwrap();
        let base_change = base_change.as_install().expect("avatars is not installing");
        assert!(base_change.is_adding_to_locked());
        assert!(base_change.to_dependencies().is_none());
        let base_pkg = base_change.install_package().expect("no package");
        assert_eq!(base_pkg.name(), "com.vrchat.base");
        assert_eq!(base_pkg.version(), &Version::new(1, 0, 0));
        assert_eq!(
            base_pkg.package_json() as *const _,
            base_package.package_json() as *const _
        );
    })
}

#[test]
fn add_to_dependencies() {
    block_on(async {
        // create minimum project
        let project = VirtualProjectBuilder::new().build().await.unwrap();

        // create package collection
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

        let avatars_change = result.package_changes().get("com.vrchat.avatars").unwrap();
        let avatars_change = avatars_change
            .as_install()
            .expect("avatars is not installing");
        assert!(avatars_change.is_adding_to_locked());
        let avatars_range = avatars_change.to_dependencies().unwrap();
        assert_eq!(
            avatars_range,
            &DependencyRange::version(Version::new(1, 0, 0))
        );
        let avatars_pkg = avatars_change.install_package().expect("no package");
        assert_eq!(avatars_pkg.name(), "com.vrchat.avatars");
        assert_eq!(avatars_pkg.version(), &Version::new(1, 0, 0));
        assert_eq!(
            avatars_pkg.package_json() as *const _,
            avatars_package.package_json() as *const _
        );

        let base_change = result.package_changes().get("com.vrchat.base").unwrap();
        let base_change = base_change.as_install().expect("avatars is not installing");
        assert!(base_change.is_adding_to_locked());
        assert!(base_change.to_dependencies().is_none());
        let base_pkg = base_change.install_package().expect("no package");
        assert_eq!(base_pkg.name(), "com.vrchat.base");
        assert_eq!(base_pkg.version(), &Version::new(1, 0, 0));
        assert_eq!(
            base_pkg.package_json() as *const _,
            base_package.package_json() as *const _
        );
    })
}

#[test]
fn install_already_installed_in_locked_to_locked() {
    block_on(async {
        // create minimum project
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

        // create package collection
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
        // create minimum project
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

        // create package collection
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

        let base_change = result.package_changes().get("com.vrchat.base").unwrap();
        let base_change = base_change.as_install().expect("base is not installing");
        assert!(!base_change.is_adding_to_locked());
        let base_range = base_change.to_dependencies().unwrap();
        assert_eq!(base_range, &DependencyRange::version(Version::new(1, 0, 0)));
        assert!(base_change.install_package().is_none());
    })
}

#[test]
fn install_already_installed_in_dependencies_to_dependencies() {
    block_on(async {
        // create minimum project
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

        // create package collection
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
        // create minimum project
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

        // create package collection
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

        let package_change = result
            .package_changes()
            .get("com.anatawa12.package")
            .unwrap();
        let package_change = package_change
            .as_install()
            .expect("package is not installing");
        assert!(package_change.is_adding_to_locked());
        assert!(package_change.to_dependencies().is_none());
        let package_pkg = package_change.install_package().expect("no package");
        assert_eq!(package_pkg.name(), "com.anatawa12.package");
        assert_eq!(package_pkg.version(), &Version::new(1, 1, 0));
        assert_eq!(
            package_pkg.package_json() as *const _,
            package.package_json() as *const _
        );

        let avatars_change = result
            .package_changes()
            .get("com.anatawa12.library")
            .unwrap();
        let avatars_change = avatars_change.as_remove().expect("library is not removing");
        assert_eq!(avatars_change.reason(), RemoveReason::Unused);
    })
}

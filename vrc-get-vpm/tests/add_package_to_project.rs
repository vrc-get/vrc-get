use crate::common::{PackageCollection, VirtualFileSystem};
use futures::executor::block_on;
use vrc_get_vpm::version::{DependencyRange, Version};
use vrc_get_vpm::{PackageCollection as _, PackageJson, UnityProject, VersionSelector};

mod common;

#[test]
fn add_to_locked_only() {
    block_on(async {
        // create minimum project
        let project_fs = VirtualFileSystem::new();
        project_fs
            .add_file("Packages/vpm-manifest.json".as_ref(), br#"{}"#)
            .await
            .unwrap();
        let project = UnityProject::load(project_fs).await.unwrap();

        // create package collection
        let mut collection = PackageCollection::new();
        collection.add(
            "Packages/com.vrchat.avatars",
            PackageJson::new("com.vrchat.avatars", Version::new(1, 0, 0))
                .add_vpm_dependency("com.vrchat.base", "1.0.0".parse().unwrap()),
        );
        collection.add(
            "Packages/com.vrchat.avatars",
            PackageJson::new("com.vrchat.base", Version::new(1, 0, 0)),
        );

        let avatars_package = collection
            .find_package_by_name(
                "com.vrchat.avatars",
                VersionSelector::specific_version(&Version::new(1, 0, 0)),
            )
            .unwrap();

        let base_package = collection
            .find_package_by_name(
                "com.vrchat.base",
                VersionSelector::specific_version(&Version::new(1, 0, 0)),
            )
            .unwrap();

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
        let project_fs = VirtualFileSystem::new();
        project_fs
            .add_file("Packages/vpm-manifest.json".as_ref(), br#"{}"#)
            .await
            .unwrap();
        let project = UnityProject::load(project_fs).await.unwrap();

        // create package collection
        let mut collection = PackageCollection::new();
        collection.add(
            "Packages/com.vrchat.avatars",
            PackageJson::new("com.vrchat.avatars", Version::new(1, 0, 0))
                .add_vpm_dependency("com.vrchat.base", "1.0.0".parse().unwrap()),
        );
        collection.add(
            "Packages/com.vrchat.avatars",
            PackageJson::new("com.vrchat.base", Version::new(1, 0, 0)),
        );

        let avatars_package = collection
            .find_package_by_name(
                "com.vrchat.avatars",
                VersionSelector::specific_version(&Version::new(1, 0, 0)),
            )
            .unwrap();

        let base_package = collection
            .find_package_by_name(
                "com.vrchat.base",
                VersionSelector::specific_version(&Version::new(1, 0, 0)),
            )
            .unwrap();

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

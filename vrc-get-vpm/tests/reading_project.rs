use crate::common::VirtualProjectBuilder;
use futures::executor::block_on;
use vrc_get_vpm::version::{ReleaseType, UnityVersion};

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
        assert!(project.unity_version().is_none());
    })
}

#[test]
fn read_version_name() {
    block_on(async {
        let project = VirtualProjectBuilder::new()
            .add_dependency_range("com.anatawa12.package", "^1.0.0")
            .add_file(
                "ProjectSettings/ProjectVersion.txt",
                "m_EditorVersion: 2022.3.6f1\n\
                m_EditorVersionWithRevision: 2022.3.6f1 (b9e6e7e9fa2d)\n\
                ",
            )
            .build()
            .await
            .unwrap();

        assert_eq!(
            project.unity_version(),
            Some(UnityVersion::new(2022, 3, 6, ReleaseType::Normal, 1))
        );
    })
}

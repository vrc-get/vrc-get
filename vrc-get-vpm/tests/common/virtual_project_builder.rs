use super::get_temp_path;
use indexmap::IndexMap;
use serde_json::json;
use std::path::{Path, PathBuf};
use vrc_get_vpm::io::{DefaultProjectIo, IoTrait};
use vrc_get_vpm::unity_project::pending_project_changes::Remove;
use vrc_get_vpm::version::{Version, VersionRange};
use vrc_get_vpm::{PackageManifest, UnityProject};

pub struct VirtualProjectBuilder {
    dependencies: IndexMap<String, String>,
    locked: IndexMap<String, (Version, IndexMap<String, VersionRange>)>,
    files: IndexMap<String, String>,
    directories: Vec<String>,
    unity_version: &'static str,
    unity_revision: &'static str,
}

impl VirtualProjectBuilder {
    pub fn new() -> Self {
        Self {
            dependencies: IndexMap::new(),
            locked: IndexMap::new(),
            files: IndexMap::new(),
            directories: vec![],
            unity_version: "2019.4.31f1",
            unity_revision: "bd5abf232a62",
        }
    }

    pub fn with_unity(&mut self, version: &'static str, revision: &'static str) -> &mut Self {
        self.unity_version = version;
        self.unity_revision = revision;
        self
    }

    pub fn add_dependency(&mut self, name: &str, version: Version) -> &mut VirtualProjectBuilder {
        self.dependencies.insert(name.into(), version.to_string());
        self
    }

    pub fn add_dependency_range(
        &mut self,
        name: &str,
        version: &str,
    ) -> &mut VirtualProjectBuilder {
        self.dependencies.insert(name.into(), version.into());
        self
    }

    pub fn add_locked(
        &mut self,
        name: &str,
        version: Version,
        dependencies: &[(&str, &str)],
    ) -> &mut VirtualProjectBuilder {
        let dependencies = dependencies
            .iter()
            .map(|(name, version)| (name.to_string(), version.parse().unwrap()))
            .collect();
        self.locked.insert(name.into(), (version, dependencies));
        self
    }

    pub fn add_file(
        &mut self,
        path: impl Into<String>,
        content: impl Into<Vec<u8>>,
    ) -> &mut VirtualProjectBuilder {
        self.files
            .insert(path.into(), String::from_utf8(content.into()).unwrap());
        self
    }

    pub fn add_dir(&mut self, path: impl Into<String>) -> &mut VirtualProjectBuilder {
        self.directories.push(path.into());
        self
    }

    pub fn add_package_json(
        &mut self,
        name: &str,
        package_json: impl Into<String>,
    ) -> &mut VirtualProjectBuilder {
        self.files
            .insert(format!("Packages/{name}/package.json"), package_json.into());
        self
    }

    #[track_caller]
    pub fn build(&self) -> impl Future<Output = std::io::Result<UnityProject>> {
        self.build_impl(get_temp_path("test_projects"))
    }

    async fn build_impl(&self, project_path: PathBuf) -> std::io::Result<UnityProject> {
        let vpm_manifest = {
            let mut dependencies = serde_json::Map::new();
            for (dependency, version) in &self.dependencies {
                dependencies.insert(dependency.to_string(), json!({ "version": version }));
            }

            let mut locked = serde_json::Map::new();
            for (name, (version, dependencies)) in &self.locked {
                let mut locked_dependencies = serde_json::Map::new();
                for (dependency, range) in dependencies {
                    locked_dependencies.insert(dependency.to_string(), json!(range.to_string()));
                }
                locked.insert(
                    name.to_string(),
                    json!({
                        "version": version.to_string(),
                        "dependencies": locked_dependencies,
                    }),
                );
            }

            json!({
                "dependencies": dependencies,
                "locked": locked,
            })
        };

        match tokio::fs::remove_dir_all(&project_path).await {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Ok(()) => {}
            Err(e) => return Err(e),
        }

        tokio::fs::create_dir_all(&project_path.join("Packages")).await?;
        tokio::fs::write(
            project_path.join("Packages/vpm-manifest.json"),
            vpm_manifest.to_string().as_bytes(),
        )
        .await?;

        tokio::fs::create_dir_all(&project_path.join("ProjectSettings")).await?;
        tokio::fs::write(
            project_path.join("ProjectSettings/ProjectVersion.txt"),
            format!(
                "m_EditorVersion: {version}\n\
                m_EditorVersionWithRevision: {version} ({revision})\n\
                ",
                version = self.unity_version,
                revision = self.unity_revision,
            )
            .as_bytes(),
        )
        .await?;

        for (name, contents) in &self.files {
            tokio::fs::create_dir_all(&project_path.join(name).parent().unwrap()).await?;
            tokio::fs::write(project_path.join(name), contents).await?;
        }

        for name in &self.directories {
            tokio::fs::create_dir_all(project_path.join(name)).await?;
        }

        UnityProject::load(DefaultProjectIo::new(project_path.into())).await
    }
}

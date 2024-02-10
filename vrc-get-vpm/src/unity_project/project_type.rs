use crate::io;
use crate::io::ProjectIo;
use crate::{ProjectType, UnityProject};

impl<IO: ProjectIo> UnityProject<IO> {
    pub async fn detect_project_type(&self) -> io::Result<ProjectType> {
        if self.get_locked("com.vrchat.avatars").is_some() {
            return Ok(ProjectType::Avatars);
        } else if self.get_locked("com.vrchat.worlds").is_some() {
            return Ok(ProjectType::Worlds);
        } else if self.manifest.has_any() {
            return Ok(ProjectType::VpmStarter);
        }

        if self.has_upm_package("com.vrchat.avatars") {
            return Ok(ProjectType::UpmAvatars);
        } else if self.has_upm_package("com.vrchat.worlds") {
            return Ok(ProjectType::UpmWorlds);
        } else if self.has_upm_package("com.vrchat.base") {
            return Ok(ProjectType::UpmStarter);
        }

        // VRCSDK2.dll is for SDK2
        if file_exists(&self.io, "Assets/VRCSDK/Plugins/VRCSDK2.dll").await {
            return Ok(ProjectType::LegacySdk2);
        }

        // VRCSDK3.dll is for SDK3 Worlds
        if file_exists(&self.io, "Assets/VRCSDK/Plugins/VRCSDK3.dll").await {
            return Ok(ProjectType::LegacyWorlds);
        }

        // VRCSDK3A.dll is for SDK3 Worlds
        if file_exists(&self.io, "Assets/VRCSDK/Plugins/VRCSDK3A.dll").await {
            return Ok(ProjectType::LegacyAvatars);
        }

        async fn file_exists(io: &impl ProjectIo, path: &str) -> bool {
            io.metadata(path.as_ref())
                .await
                .map(|x| x.is_file())
                .unwrap_or(false)
        }

        Ok(ProjectType::Unknown)
    }
}

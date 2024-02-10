use crate::io;
use crate::io::ProjectIo;
use crate::{ProjectType, UnityProject};

impl<IO: ProjectIo> UnityProject<IO> {
    pub fn detect_project_type(&self) -> io::Result<ProjectType> {
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

        // TODO: add legacy project type detection with installed files
        Ok(ProjectType::Unknown)
    }
}

use crate::io::IoTrait;
use crate::{ProjectType, UnityProject};

impl UnityProject {
    pub async fn detect_project_type(&self) -> ProjectType {
        if self.get_locked("com.vrchat.avatars").is_some() {
            return ProjectType::Avatars;
        } else if self.get_locked("com.vrchat.worlds").is_some() {
            return ProjectType::Worlds;
        } else if self.manifest.has_any() {
            return ProjectType::VpmStarter;
        }

        if self.has_upm_package("com.vrchat.avatars") {
            return ProjectType::UpmAvatars;
        } else if self.has_upm_package("com.vrchat.worlds") {
            return ProjectType::UpmWorlds;
        } else if self.has_upm_package("com.vrchat.base") {
            return ProjectType::UpmStarter;
        }

        // VRCSDK2.dll is for SDK2
        if self
            .io
            .is_file("Assets/VRCSDK/Plugins/VRCSDK2.dll".as_ref())
            .await
        {
            return ProjectType::LegacySdk2;
        }

        // VRCSDK3.dll is for SDK3 Worlds
        if self
            .io
            .is_file("Assets/VRCSDK/Plugins/VRCSDK3.dll".as_ref())
            .await
        {
            return ProjectType::LegacyWorlds;
        }

        // VRCSDK3A.dll is for SDK3 Worlds
        if self
            .io
            .is_file("Assets/VRCSDK/Plugins/VRCSDK3A.dll".as_ref())
            .await
        {
            return ProjectType::LegacyAvatars;
        }

        ProjectType::Unknown
    }
}

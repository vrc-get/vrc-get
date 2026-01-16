use std::path::Path;
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

    pub async fn detect_display_name(&self) -> Option<String> {
        let project_name_file = Path::new("UserSettings/ProjectName.txt");

        if !self.io.is_file(project_name_file.as_ref()).await {
            return None;
        }

        let mut file = self.io.open(project_name_file.as_ref()).await.ok()?;
        let mut content = String::new();
        use futures::AsyncReadExt;
        file.read_to_string(&mut content).await.ok()?;

        content.lines().next().map(|x| x.to_owned())
    }
}

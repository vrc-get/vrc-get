use super::Settings;
use crate::PackageManifest;
use crate::io::DefaultEnvironmentIo;
use crate::package_manifest::LooseManifest;
use crate::utils::try_load_json;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct UserPackageCollection {
    user_packages: Vec<(PathBuf, PackageManifest)>,
}

impl UserPackageCollection {
    pub async fn load(settings: &Settings, io: &DefaultEnvironmentIo) -> Self {
        let mut user_packages = UserPackageCollection::new();
        for x in settings.user_package_folders() {
            user_packages.try_add_package(io, x).await;
        }
        user_packages
    }

    pub fn packages(&self) -> impl Iterator<Item = &(PathBuf, PackageManifest)> {
        self.user_packages.iter()
    }

    pub(crate) fn new() -> UserPackageCollection {
        Self {
            user_packages: Vec::new(),
        }
    }

    pub(crate) async fn try_add_package(&mut self, io: &DefaultEnvironmentIo, folder: &Path) {
        match try_load_json::<LooseManifest>(io, &folder.join("package.json")).await {
            Ok(Some(LooseManifest(package_json))) => {
                self.user_packages.push((folder.to_owned(), package_json));
            }
            Ok(None) => {
                log::warn!("package.json not found in {}", folder.display());
            }
            Err(e) => {
                log::warn!("Failed to load package.json in {}: {e}", folder.display());
            }
        }
    }

    pub(crate) fn into_packages(self) -> Vec<(PathBuf, PackageManifest)> {
        self.user_packages
    }
}

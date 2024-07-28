use crate::io::EnvironmentIo;
use crate::package_manifest::LooseManifest;
use crate::utils::try_load_json;
use crate::PackageManifest;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct UserPackageCollection {
    user_packages: Vec<(PathBuf, PackageManifest)>,
}

impl UserPackageCollection {
    pub(crate) fn new() -> UserPackageCollection {
        Self {
            user_packages: Vec::new(),
        }
    }

    pub(crate) async fn try_add_package(&mut self, io: &impl EnvironmentIo, folder: &Path) {
        match try_load_json::<LooseManifest>(io, &folder.join("package.json")).await {
            Ok(Some(LooseManifest(package_json))) => {
                self.user_packages.push((folder.to_owned(), package_json));
            }
            Ok(None) => {
                log::warn!("package.json not found in {}", folder.display());
            }
            Err(e) => {
                log::warn!("Failed to load package.json in {}: {}", folder.display(), e);
            }
        }
    }

    pub(crate) fn into_packages(self) -> Vec<(PathBuf, PackageManifest)> {
        self.user_packages
    }
}

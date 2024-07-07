use crate::io::EnvironmentIo;
use crate::package_manifest::LooseManifest;
use crate::utils::try_load_json;
use crate::{PackageCollection, PackageInfo, PackageManifest, VersionSelector};
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

    pub(crate) fn clear(&mut self) {
        self.user_packages.clear();
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

    pub(crate) fn get_packages(&self) -> &[(PathBuf, PackageManifest)] {
        &self.user_packages
    }

    pub(crate) fn add_user_package(&mut self, path: PathBuf, json: PackageManifest) {
        self.user_packages.push((path, json));
    }

    pub(crate) fn remove_user_package(&mut self, path: &Path) {
        self.user_packages.retain(|(p, _)| p != path);
    }
}

impl PackageCollection for UserPackageCollection {
    fn get_all_packages(&self) -> impl Iterator<Item = PackageInfo> {
        self.user_packages
            .iter()
            .map(|(path, json)| PackageInfo::local(json, path))
    }

    fn find_packages(&self, package: &str) -> impl Iterator<Item = PackageInfo> {
        self.user_packages
            .iter()
            .filter(move |(_, json)| json.name() == package)
            .map(|(path, json)| PackageInfo::local(json, path))
    }

    fn find_package_by_name(
        &self,
        package: &str,
        package_selector: VersionSelector,
    ) -> Option<PackageInfo> {
        self.find_packages(package)
            .filter(|x| package_selector.satisfies(x.package_json()))
            .max_by_key(|x| x.version())
    }
}

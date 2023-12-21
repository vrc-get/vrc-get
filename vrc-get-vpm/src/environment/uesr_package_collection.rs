use crate::structs::package::PackageJson;
use crate::{load_json_or_default, PackageCollection, PackageInfo, VersionSelector};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct UserPackageCollection {
    user_packages: Vec<(PathBuf, PackageJson)>,
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

    pub(crate) async fn try_add_package(&mut self, folder: &Path) -> io::Result<()> {
        if let Some(package_json) =
            load_json_or_default::<Option<PackageJson>>(&folder.join("package.json")).await?
        {
            self.user_packages.push((folder.to_owned(), package_json));
        }
        Ok(())
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
            .filter(move |(_, json)| json.name == package)
            .map(|(path, json)| PackageInfo::local(json, path))
    }

    fn find_package_by_name(
        &self,
        package: &str,
        package_selector: VersionSelector,
    ) -> Option<PackageInfo> {
        self.find_packages(package)
            .filter(|x| package_selector.satisfies(x))
            .max_by_key(|x| x.version())
    }
}

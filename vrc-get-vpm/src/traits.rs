use crate::repository::local::LocalCachedRepository;
use crate::structs::package::PackageJson;
use crate::{Environment, PackageInfo, PackageSelector};
use core::iter::Iterator;
use core::option::Option;
use std::io;
use tokio::fs::File;

mod seal {
    pub trait Sealed {}
}

pub trait PackageCollection: seal::Sealed {
    fn get_all_packages(&self) -> impl Iterator<Item = PackageInfo>;

    fn find_packages(&self, package: &str) -> impl Iterator<Item = PackageInfo>;

    fn find_package_by_name(
        &self,
        package: &str,
        package_selector: PackageSelector,
    ) -> Option<PackageInfo>;
}

/// The trait for downloading remote packages.
///
/// Caching packages is responsibility of this crate.
pub trait RemotePackageDownloader: seal::Sealed {
    fn get_package(
        &self,
        repository: &LocalCachedRepository,
        package: &PackageJson,
    ) -> impl std::future::Future<Output = io::Result<File>> + Send;
}

impl seal::Sealed for Environment {}
impl seal::Sealed for LocalCachedRepository {}
impl seal::Sealed for crate::environment::UserPackageCollection {}

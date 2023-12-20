use crate::repository::local::LocalCachedRepository;
use crate::{Environment, PackageInfo, PackageSelector};
use core::iter::Iterator;
use core::option::Option;

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

impl seal::Sealed for Environment {}
impl seal::Sealed for LocalCachedRepository {}
impl seal::Sealed for crate::environment::UserPackageCollection {}

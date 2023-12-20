use crate::{Environment, PackageInfo, PackageSelector};

mod seal {
    pub trait Sealed {}
}

pub trait PackageCollection : seal::Sealed {
    fn find_package_by_name(
        &self,
        package: &str,
        package_selector: PackageSelector,
    ) -> Option<PackageInfo>;
}

impl seal::Sealed for Environment {
}

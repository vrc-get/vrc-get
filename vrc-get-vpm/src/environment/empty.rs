use std::future::Future;
use std::io;
use tokio::fs::File;
use crate::{PackageJson, RemotePackageDownloader};
use crate::repository::local::LocalCachedRepository;

/// The enviroment that holds no packages
/// 
/// This will be used for removing packages.
pub struct EmptyEnvironment;

impl RemotePackageDownloader for EmptyEnvironment {
    fn get_package(
        &self,
        _repository: &LocalCachedRepository,
        _package: &PackageJson,
    ) -> impl Future<Output = io::Result<File>> + Send {
        futures::future::err(io::Error::new(
            io::ErrorKind::NotFound,
            "not found",
        ))
    }
}

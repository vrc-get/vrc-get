use crate::io;
use crate::io::DefaultEnvironmentIo;
use crate::io::SeekFrom;
use crate::repository::local::LocalCachedRepository;
use crate::traits::EnvironmentIoHolder;
use crate::{PackageJson, RemotePackageDownloader};
use futures::{AsyncRead, AsyncSeek};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// The enviroment that holds no packages
///
/// This will be used for removing packages.
pub struct EmptyEnvironment;

impl crate::traits::seal::Sealed for EmptyEnvironment {}

impl RemotePackageDownloader for EmptyEnvironment {
    type FileStream = NoFileStream;

    fn get_package(
        &self,
        _repository: &LocalCachedRepository,
        _package: &PackageJson,
    ) -> impl Future<Output = io::Result<Self::FileStream>> + Send {
        futures::future::err(io::Error::new(io::ErrorKind::NotFound, "not found"))
    }
}

impl EnvironmentIoHolder for EmptyEnvironment {
    type EnvironmentIo = DefaultEnvironmentIo;

    fn io(&self) -> &Self::EnvironmentIo {
        panic!("EmptyEnvironment::io() should not be called")
    }
}

pub enum NoFileStream {}

impl AsyncRead for NoFileStream {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match *self {}
    }
}

impl AsyncSeek for NoFileStream {
    fn poll_seek(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _pos: SeekFrom,
    ) -> Poll<io::Result<u64>> {
        match *self {}
    }
}

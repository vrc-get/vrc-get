mod copy_recursive;
mod extract_zip;
mod sha256_async_write;

use async_zip::error::ZipError;
use either::Either;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, Stream, StreamExt, TryStream};
use pin_project_lite::pin_project;
use serde_json::{Map, Value};
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use tokio::fs::{read_dir, DirEntry, ReadDir};

pub(crate) use copy_recursive::copy_recursive;
pub(crate) use extract_zip::extract_zip;
pub(crate) use sha256_async_write::Sha256AsyncWrite;

pub(crate) trait PathBufExt {
    fn joined(self, into: impl AsRef<Path>) -> Self;
}

impl PathBufExt for PathBuf {
    fn joined(mut self, into: impl AsRef<Path>) -> Self {
        self.push(into);
        self
    }
}

pub(crate) trait MapResultExt<T> {
    type Output;
    /// returns
    fn err_mapped(self) -> Result<T, Self::Output>;
}

impl<T> MapResultExt<T> for Result<T, reqwest::Error> {
    type Output = io::Error;

    fn err_mapped(self) -> Result<T, Self::Output> {
        self.map_err(|err| io::Error::new(io::ErrorKind::NotFound, err))
    }
}

impl<T> MapResultExt<T> for Result<T, ZipError> {
    type Output = io::Error;

    fn err_mapped(self) -> Result<T, Self::Output> {
        self.map_err(|err| {
            use io::ErrorKind::*;
            let kind = match err {
                ZipError::FeatureNotSupported(_) => Unsupported,
                ZipError::CompressionNotSupported(_) => Unsupported,
                ZipError::AttributeCompatibilityNotSupported(_) => Unsupported,
                ZipError::TargetZip64NotSupported => Unsupported,
                ZipError::EOFNotReached => InvalidData,
                ZipError::UnableToLocateEOCDR => InvalidData, // better kind?
                ZipError::InvalidExtraFieldHeader(_, _) => InvalidData,
                ZipError::Zip64ExtendedFieldIncomplete => InvalidData,
                ZipError::UpstreamReadError(ref upstream) => upstream.kind(),
                ZipError::CRC32CheckError => InvalidData,
                ZipError::EntryIndexOutOfBounds => InvalidData,
                ZipError::UnexpectedHeaderError(_, _) => InvalidData,
                // fallback to InvalidData but data
                _ => InvalidData,
            };

            io::Error::new(kind, err)
        })
    }
}

pub(crate) trait JsonMapExt {
    fn get_or_put_mut<Q, V>(&mut self, key: Q, value: impl FnOnce() -> V) -> &mut Value
    where
        Q: Into<String>,
        V: Into<Value>;
}

impl JsonMapExt for Map<String, Value> {
    fn get_or_put_mut<Q, V>(&mut self, key: Q, value: impl FnOnce() -> V) -> &mut Value
    where
        Q: Into<String>,
        V: Into<Value>,
    {
        self.entry(key.into()).or_insert_with(|| value().into())
    }
}

pub(crate) trait OurTryStreamExt: Stream + Sized {
    fn flatten_ok(self) -> FlattenOk<Self>
    where
        Self: TryStream,
        Self::Ok: Stream,
    {
        FlattenOk {
            stream: self,
            next: None,
        }
    }
}

impl<T: Stream + Sized> OurTryStreamExt for T {}

pin_project! {
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub(crate) struct FlattenOk<S> where S: TryStream{
        #[pin]
        stream: S,
        #[pin]
        next: Option<S::Ok>,
    }
}

impl<S> Stream for FlattenOk<S>
where
    S: TryStream,
    S::Ok: Stream,
{
    type Item = Result<<S::Ok as Stream>::Item, S::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        Poll::Ready(loop {
            if let Some(s) = this.next.as_mut().as_pin_mut() {
                if let Some(item) = ready!(s.poll_next(cx)) {
                    break Some(Ok(item));
                } else {
                    this.next.set(None);
                }
            } else if let Some(s) = ready!(this.stream.as_mut().try_poll_next(cx)?) {
                this.next.set(Some(s));
            } else {
                break None;
            }
        })
    }
}

pub(crate) struct WalkDirEntry {
    pub(crate) original: DirEntry,
    pub(crate) relative: PathBuf,
}

impl WalkDirEntry {
    pub(crate) fn new(original: DirEntry, relative: PathBuf) -> Self {
        Self { original, relative }
    }

    pub(crate) fn path(&self) -> PathBuf {
        self.original.path()
    }
}

pub(crate) fn walk_dir_relative(
    root: &Path,
    paths: impl IntoIterator<Item = PathBuf>,
) -> impl Stream<Item = WalkDirEntry> {
    type FutureResult =
        Either<io::Result<(ReadDir, PathBuf)>, io::Result<Option<(ReadDir, PathBuf, DirEntry)>>>;

    async fn read_dir_phase(
        absolute: PathBuf,
        relative: PathBuf,
    ) -> io::Result<(ReadDir, PathBuf)> {
        Ok((read_dir(absolute).await?, relative))
    }

    async fn next_phase(
        mut read_dir: ReadDir,
        relative: PathBuf,
    ) -> io::Result<Option<(ReadDir, PathBuf, DirEntry)>> {
        if let Some(entry) = read_dir.next_entry().await? {
            Ok(Some((read_dir, relative, entry)))
        } else {
            Ok(None)
        }
    }

    let mut futures = FuturesUnordered::new();

    for path in paths {
        futures.push(Either::Left(
            read_dir_phase(root.join(&path), path).map(FutureResult::Left),
        ));
    }

    async_stream::stream! {
        loop {
            match futures.next().await {
                None => break,
                Some(Either::Left(Err(_))) => continue,
                Some(Either::Left(Ok((read_dir, dir_relative)))) => {
                    futures.push(Either::Right(next_phase(read_dir, dir_relative).map(FutureResult::Right)))
                },
                Some(Either::Right(Err(_))) => continue,
                Some(Either::Right(Ok(None))) => continue,
                Some(Either::Right(Ok(Some((read_dir_iter, dir_relative, entry))))) => {
                    let new_relative_path = dir_relative.join(entry.file_name());
                    futures.push(Either::Left(read_dir_phase(entry.path(), new_relative_path.clone()).map(FutureResult::Left)));
                    futures.push(Either::Right(next_phase(read_dir_iter, dir_relative).map(FutureResult::Right)));
                    yield WalkDirEntry::new(entry, new_relative_path);
                },
            }
        }
    }
}

#[cfg(feature = "experimental-yank")]
pub(crate) fn is_truthy(value: Option<&Value>) -> bool {
    // see https://developer.mozilla.org/en-US/docs/Glossary/Falsy
    match value {
        Some(Value::Null) => false,
        None => false,
        Some(Value::Bool(false)) => false,
        // No NaN in json
        Some(Value::Number(num)) if num.as_f64() == Some(0.0) => false,
        Some(Value::String(s)) if s.is_empty() => false,
        _ => true,
    }
}

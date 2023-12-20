mod copy_recursive;
mod extract_zip;
mod sha256_async_write;

use async_zip::error::ZipError;
use futures::stream::FuturesUnordered;
use futures::{Stream, TryStream};
use pin_project_lite::pin_project;
use serde_json::{Map, Value};
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::Poll::Ready;
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

macro_rules! parse_hex_bits {
    ($name: ident : $bits: literal) => {
        pub(crate) fn $name(hex: [u8; $bits / 4]) -> Option<[u8; $bits / 8]> {
            let mut result = [0u8; $bits / 8];
            for i in 0..($bits / 8) {
                let upper = match hex[i * 2 + 0] {
                    c @ b'0'..=b'9' => c - b'0',
                    c @ b'a'..=b'f' => c - b'a' + 10,
                    c @ b'A'..=b'F' => c - b'A' + 10,
                    _ => return None,
                };
                let lower = match hex[i * 2 + 1] {
                    c @ b'0'..=b'9' => c - b'0',
                    c @ b'a'..=b'f' => c - b'a' + 10,
                    c @ b'A'..=b'F' => c - b'A' + 10,
                    _ => return None,
                };
                result[i] = upper << 4 | lower;
            }
            Some(result)
        }
    };
}

parse_hex_bits!(parse_hex_256: 256);
parse_hex_bits!(parse_hex_128: 128);

pub(crate) fn walk_dir(paths: impl IntoIterator<Item = PathBuf>) -> impl Stream<Item = DirEntry> {
    pin_project! {
        #[project = ReadingDirProj]
        enum ReadingDir<ReadDirFut>
            where ReadDirFut: Future<Output = io::Result<ReadDir>>,
        {
            ReadDir{
                #[pin]
                inner: ReadDirFut,
            },
            ReadDirNext {
                inner: Option<ReadDir>,
            },
        }
    }

    enum ReadingDirResult {
        ReadDir(io::Result<ReadDir>),
        ReadDirNext(io::Result<Option<(ReadDir, DirEntry)>>),
    }

    impl<ReadDirFut> Future for ReadingDir<ReadDirFut>
    where
        ReadDirFut: Future<Output = io::Result<ReadDir>>,
    {
        type Output = ReadingDirResult;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            match self.project() {
                ReadingDirProj::ReadDir { inner } => {
                    Ready(ReadingDirResult::ReadDir(ready!(inner.poll(cx))))
                }
                ReadingDirProj::ReadDirNext { inner } => Ready(ReadingDirResult::ReadDirNext(
                    ready!(inner.as_mut().unwrap().poll_next_entry(cx))
                        .map(|x| x.map(|y| (inner.take().unwrap(), y))),
                )),
            }
        }
    }

    pin_project! {
        struct StreamImpl<ReadDirFut, ReadDirFn>
            where ReadDirFut: Future<Output = io::Result<ReadDir>>,
                  ReadDirFn: Fn(PathBuf) -> ReadDirFut,
        {
            #[pin]
            inner: FuturesUnordered<ReadingDir<ReadDirFut>>,
            read_dir: ReadDirFn,
        }
    }

    impl<ReadDirFut, ReadDirFn> StreamImpl<ReadDirFut, ReadDirFn>
    where
        ReadDirFut: Future<Output = io::Result<ReadDir>>,
        ReadDirFn: Fn(PathBuf) -> ReadDirFut,
    {
        fn new(read_dir: ReadDirFn, paths: impl Iterator<Item = PathBuf>) -> Self {
            let futures = FuturesUnordered::new();
            for path in paths {
                futures.push(ReadingDir::ReadDir {
                    inner: read_dir(path),
                });
            }
            Self {
                inner: futures,
                read_dir,
            }
        }
    }

    impl<ReadDirFut, ReadDirFn> Stream for StreamImpl<ReadDirFut, ReadDirFn>
    where
        ReadDirFut: Future<Output = io::Result<ReadDir>>,
        ReadDirFn: Fn(PathBuf) -> ReadDirFut,
    {
        type Item = DirEntry;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            loop {
                match ready!(self.as_mut().project().inner.poll_next(cx)) {
                    None => return Ready(None),
                    Some(ReadingDirResult::ReadDir(Err(_))) => continue,
                    Some(ReadingDirResult::ReadDir(Ok(read_dir))) => {
                        self.inner.push(ReadingDir::ReadDirNext {
                            inner: Some(read_dir),
                        })
                    }
                    Some(ReadingDirResult::ReadDirNext(Err(_))) => continue,
                    Some(ReadingDirResult::ReadDirNext(Ok(None))) => continue,
                    Some(ReadingDirResult::ReadDirNext(Ok(Some((read_dir, entry))))) => {
                        self.inner.push(ReadingDir::ReadDir {
                            inner: (self.read_dir)(entry.path()),
                        });
                        self.inner.push(ReadingDir::ReadDirNext {
                            inner: Some(read_dir),
                        });
                        return Ready(Some(entry));
                    }
                }
            }
        }
    }

    StreamImpl::new(read_dir, paths.into_iter())
}

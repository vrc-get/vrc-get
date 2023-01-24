use async_zip::error::ZipError;
use futures::{Stream, TryStream};
use pin_project_lite::pin_project;
use serde_json::{Map, Value};
use std::hint::unreachable_unchecked;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{ready, Context, Poll};

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

pub(crate) fn error_flatten<I, E>(r: Result<I, E>) -> ErrorFlatten<I::IntoIter, E>
where
    I: IntoIterator,
{
    ErrorFlatten {
        inner: match r {
            Ok(i) => ErrorFlattenInner::Iter(i.into_iter()),
            Err(e) => ErrorFlattenInner::Err(e),
        },
    }
}

pub(crate) struct ErrorFlatten<I, E> {
    inner: ErrorFlattenInner<I, E>,
}

enum ErrorFlattenInner<I, E> {
    Err(E),
    Iter(I),
    End,
}

impl<I, E> Iterator for ErrorFlatten<I, E>
where
    I: Iterator,
{
    type Item = Result<I::Item, E>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner {
            ErrorFlattenInner::Err(_) => {
                match std::mem::replace(&mut self.inner, ErrorFlattenInner::End) {
                    ErrorFlattenInner::Err(e) => Some(Err(e)),
                    _ => unsafe { unreachable_unchecked() },
                }
            }
            ErrorFlattenInner::Iter(ref mut i) => i.next().map(Ok),
            ErrorFlattenInner::End => None,
        }
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

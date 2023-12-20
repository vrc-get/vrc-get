use pin_project_lite::pin_project;
use sha2::digest::Output;
use sha2::{Digest, Sha256};
use std::io::Error;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use tokio::io::AsyncWrite;

pin_project! {

    pub(crate) struct Sha256AsyncWrite<W: AsyncWrite> {
        #[pin]
        inner: W,
        hasher: Sha256,
    }
}

impl<W: AsyncWrite> Sha256AsyncWrite<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            hasher: Sha256::default(),
        }
    }

    pub fn finalize(self) -> (W, Output<Sha256>) {
        (self.inner, self.hasher.finalize())
    }
}

impl<W: AsyncWrite> AsyncWrite for Sha256AsyncWrite<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.project();
        let size = ready!(this.inner.poll_write(cx, buf))?;
        this.hasher.update(&buf[..size]);
        Poll::Ready(Ok(size))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.project().inner.poll_shutdown(cx)
    }
}

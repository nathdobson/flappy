
use core::fmt::{Display, Formatter};
use embedded_io_async::{BufRead, ErrorKind, ErrorType, Read, Write};
use tokio::io;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite};
#[derive(Debug)]
pub struct TokioStreamAdapter<T>(pub T);

#[derive(Debug)]
pub struct TokioErrorAdapter(pub io::Error);

impl<T> ErrorType for TokioStreamAdapter<T> {
    type Error = TokioErrorAdapter;
}

impl core::error::Error for TokioErrorAdapter {}

impl Display for TokioErrorAdapter {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl embedded_io_async::Error for TokioErrorAdapter {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

impl<T: Unpin + AsyncWrite> Write for TokioStreamAdapter<T> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.0.write(buf).await.map_err(TokioErrorAdapter)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush().await.map_err(TokioErrorAdapter)
    }
}

impl<T: Unpin + AsyncRead> Read for TokioStreamAdapter<T> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.0.read(buf).await.map_err(TokioErrorAdapter)
    }
}

impl<T: Unpin + AsyncBufRead> BufRead for TokioStreamAdapter<T> {
    async fn fill_buf(&mut self) -> Result<&[u8], Self::Error> {
        self.0.fill_buf().await.map_err(TokioErrorAdapter)
    }

    fn consume(&mut self, amt: usize) {
        self.0.consume(amt)
    }
}

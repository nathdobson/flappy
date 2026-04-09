use cortex_m::prelude::_embedded_hal_blocking_spi_Write;
use embassy_net::tcp::{TcpReader, TcpWriter};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_io::ErrorType;
use embedded_io_async::{Read, Write};

/// A wrapper that combines one implementation of [embedded_io_async::Read] and one implementation of [embedded_io_async::Write] into a single value.
pub struct MergeSocket<W, R> {
    write: Mutex<NoopRawMutex, W>,
    read: Mutex<NoopRawMutex, R>,
}

impl<W, R> MergeSocket<W, R> {
    pub fn new(write: W, read: R) -> MergeSocket<W, R> {
        MergeSocket {
            write: Mutex::new(write),
            read: Mutex::new(read),
        }
    }
}

impl<'a, W: ErrorType, R: ErrorType<Error = W::Error>> ErrorType for &'a MergeSocket<W, R> {
    type Error = W::Error;
}

impl<'a, W: Write, R: ErrorType<Error = W::Error>> Write for &'a MergeSocket<W, R> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.write.lock().await.write(buf).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.write.lock().await.flush().await
    }
}

impl<'a, W: ErrorType, R: Read + ErrorType<Error = W::Error>> Read for &'a MergeSocket<W, R> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.read.lock().await.read(buf).await
    }
}

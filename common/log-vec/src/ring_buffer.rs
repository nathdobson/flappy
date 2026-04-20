use crate::CapacityError;
use crate::slice_pair::SlicePair;
use core::fmt::{Debug, Formatter};
use embedded_io::{BufRead, ErrorType, Read, Write};

pub trait RingBufferStorage: AsRef<[u8]> + AsMut<[u8]> + Debug + sealed::RingBufferStorageSealed {}
mod sealed{
    pub trait RingBufferStorageSealed {}
    impl<const N: usize> RingBufferStorageSealed for [u8; N] {}
    impl RingBufferStorageSealed for [u8] {}

}

pub struct RingBufferInner<T: ?Sized> {
    start: usize,
    len: usize,
    data: T,
}

pub type RingBuffer<const N: usize> = RingBufferInner<[u8; N]>;
pub type RingBufferView = RingBufferInner<[u8]>;

impl<const N: usize> RingBufferStorage for [u8; N] {}
impl RingBufferStorage for [u8] {}


impl<const N: usize> RingBuffer<N> {
    pub fn new() -> Self {
        RingBuffer {
            start: 0,
            len: 0,
            data: [0; N],
        }
    }
}

impl<T: ?Sized + RingBufferStorage> RingBufferInner<T> {
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn as_slice_pair(&self) -> SlicePair<'_, u8> {
        SlicePair::new(
            &self.data.as_ref()[self.start..],
            &self.data.as_ref()[..self.start],
        )
        .subslice(0..self.len)
        .unwrap()
    }
    pub fn consume_back(&mut self, len: usize) {
        self.len -= len;
    }
}

impl<T: ?Sized + RingBufferStorage> ErrorType for RingBufferInner<T> {
    type Error = CapacityError;
}

impl<T: ?Sized + RingBufferStorage> Write for RingBufferInner<T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        if buf.len() == 0 {
            return Ok(0);
        }
        if self.len == self.data.as_ref().len() {
            return Err(CapacityError);
        }
        let write_start;
        let write_len;
        if self.start + self.len >= self.data.as_ref().len() {
            write_start = self.start + self.len - self.data.as_ref().len();
            write_len = (self.start - write_start).min(buf.len());
        } else {
            write_start = self.start + self.len;
            write_len = (self.data.as_ref().len() - write_start).min(buf.len());
        }
        self.data.as_mut()[write_start..write_start + write_len].copy_from_slice(&buf[..write_len]);
        self.len += write_len;
        Ok(write_len)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T: ?Sized + RingBufferStorage> Read for RingBufferInner<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let filled = self.fill_buf()?;
        let len = filled.len().min(buf.len());
        buf[..len].copy_from_slice(&filled[..len]);
        self.consume(len);
        Ok(len)
    }
}

impl<T: ?Sized + RingBufferStorage> BufRead for RingBufferInner<T> {
    fn fill_buf(&mut self) -> Result<&[u8], Self::Error> {
        let data = self.data.as_mut();
        Ok(&data[self.start..(self.start + self.len).min(data.len())])
    }
    fn consume(&mut self, amt: usize) {
        assert!(amt <= self.len);
        self.start += amt;
        self.len -= amt;
        if self.start >= self.data.as_ref().len() {
            self.start -= self.data.as_ref().len();
        }
    }
}

impl<T: ?Sized + RingBufferStorage> Debug for RingBufferInner<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RingBuffer")
            .field("start", &self.start)
            .field("len", &self.len)
            .field("data", &&self.data)
            .field_with("sequence", |f| f.debug_list().entries(self).finish())
            .finish()
    }
}

impl<'a, T: ?Sized + RingBufferStorage> IntoIterator for &'a RingBufferInner<T> {
    type Item = &'a u8;
    type IntoIter = impl 'a + Iterator<Item = &'a u8>;
    fn into_iter(self) -> Self::IntoIter {
        let data = self.data.as_ref();
        data.as_ref()[self.start..]
            .iter()
            .chain(data.iter())
            .take(self.len)
    }
}

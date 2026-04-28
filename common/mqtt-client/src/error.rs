use core::alloc::AllocError;
use core::fmt::{Debug, Display, Formatter, write};
use heapless::CapacityError;
use mqtt_core::error::ProtocolError;
use mqtt_core::protocol::ReasonCode;

#[derive(Debug)]
pub enum Error<W, R> {
    WriteError(W),
    ReadError(R),
    ProtocolError(ProtocolError),
}

impl<W: Debug + Display, R: Debug + Display> core::error::Error for Error<W, R> {}

// impl From<CapacityError> for ProtocolError {
//     fn from(value: CapacityError) -> Self {
//         ProtocolError::BufferFull
//     }
// }
//
// impl From<AllocError> for ProtocolError {
//     fn from(value: AllocError) -> Self {
//         ProtocolError::BufferFull
//     }
// }

impl<W, R> From<CapacityError> for Error<W, R> {
    fn from(value: CapacityError) -> Self {
        Error::ProtocolError(ProtocolError::BufferFull)
    }
}

impl<W, R> From<AllocError> for Error<W, R> {
    fn from(value: AllocError) -> Self {
        Error::ProtocolError(ProtocolError::BufferFull)
    }
}

impl<W, R> Error<W, R> {
    pub fn map_write<W2>(self, f: impl FnOnce(W) -> W2) -> Error<W2, R> {
        match self {
            Error::WriteError(w) => Error::WriteError(f(w)),
            Error::ReadError(r) => Error::ReadError(r),
            Error::ProtocolError(p) => Error::ProtocolError(p),
        }
    }
    pub fn map_read<R2>(self, f: impl FnOnce(R) -> R2) -> Error<W, R2> {
        match self {
            Error::WriteError(w) => Error::WriteError(w),
            Error::ReadError(r) => Error::ReadError(f(r)),
            Error::ProtocolError(p) => Error::ProtocolError(p),
        }
    }
}

impl<W, R> From<ProtocolError> for Error<W, R> {
    fn from(value: ProtocolError) -> Self {
        Error::ProtocolError(value)
    }
}

impl<W: Display, R: Display> Display for Error<W, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::WriteError(w) => write!(f, "write error: {}", w),
            Error::ReadError(r) => write!(f, "read error: {}", r),
            Error::ProtocolError(p) => write!(f, "protocol error: {}", p),
        }
    }
}

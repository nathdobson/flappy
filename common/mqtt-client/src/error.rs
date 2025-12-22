use mqtt_core::protocol::ReasonCode;
use core::alloc::AllocError;
use core::fmt::{Display, Formatter};
use heapless::CapacityError;
use mqtt_core::error::ProtocolError;

#[derive(Debug)]
pub enum Error<E> {
    NetworkError(E),
    ProtocolError(ProtocolError),
}

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

impl<E> From<CapacityError> for Error<E> {
    fn from(value: CapacityError) -> Self {
        Error::ProtocolError(ProtocolError::BufferFull)
    }
}

impl<E> From<AllocError> for Error<E> {
    fn from(value: AllocError) -> Self {
        Error::ProtocolError(ProtocolError::BufferFull)
    }
}

impl<E> Error<E> {
    pub fn map<E2>(self, f: impl FnOnce(E) -> E2) -> Error<E2> {
        match self {
            Error::NetworkError(e) => Error::NetworkError(f(e)),
            Error::ProtocolError(e) => Error::ProtocolError(e),
        }
    }
}

impl<E> From<ProtocolError> for Error<E> {
    fn from(value: ProtocolError) -> Self {
        Error::ProtocolError(value)
    }
}

impl<E: Display> Display for Error<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::NetworkError(e) => write!(f, "{}", e),
            Error::ProtocolError(e) => write!(f, "{}", e),
        }
    }
}


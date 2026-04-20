use core::fmt::{Display, Formatter};
use embedded_io::ErrorKind;

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Default)]
pub struct CapacityError;

impl embedded_io::Error for CapacityError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::OutOfMemory
    }
}

impl core::error::Error for CapacityError {}

impl Display for CapacityError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "log is full")
    }
}

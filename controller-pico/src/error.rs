use core::fmt::{Display, Formatter};
use embassy_executor::SpawnError;
use trouble_host::{codec, BleHostError};

#[derive(Debug)]
pub enum Error {
    SpawnError(SpawnError),
    UuidError(uuid::Error),
    TroubleError(trouble_host::Error),
    TroubleCodecError(codec::Error),
    BleHostError(BleHostError<cyw43::bluetooth::Error>),
}

pub type Result<T> = core::result::Result<T, Error>;

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::SpawnError(error) => write!(f, "Spawn error: {}", error),
            Error::UuidError(error) => write!(f, "Uuid error: {}", error),
            Error::TroubleError(error) => write!(f, "Trouble error: {:?}", error),
            Error::TroubleCodecError(error) => write!(f, "Trouble codec error: {:?}", error),
            Error::BleHostError(error) => write!(f, "Ble host error: {:?}", error),
        }
    }
}

impl From<SpawnError> for Error {
    fn from(error: SpawnError) -> Self {
        Error::SpawnError(error)
    }
}

impl From<uuid::Error> for Error {
    fn from(error: uuid::Error) -> Self {
        Error::UuidError(error)
    }
}

impl From<trouble_host::Error> for Error {
    fn from(error: trouble_host::Error) -> Self {
        Error::TroubleError(error)
    }
}

impl From<codec::Error> for Error {
    fn from(error: codec::Error) -> Self {
        Error::TroubleCodecError(error)
    }
}

impl From<BleHostError<cyw43::bluetooth::Error>> for Error {
    fn from(error: BleHostError<cyw43::bluetooth::Error>) -> Self {
        Error::BleHostError(error)
    }
}

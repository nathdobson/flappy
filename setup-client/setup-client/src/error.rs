use itertools::ExactlyOneError;
use nusb::ActiveConfigurationError;
use serde_json_core::heapless::CapacityError;
use serde_string::{StringDeserializerError, StringSerializerError};

#[derive(Debug)]
pub enum Error {
    UsbError(nusb::Error),
    ActiveConfigurationError(ActiveConfigurationError),
    IoError(std::io::Error),
    MissingEndpoint,
    MissingInterface,
    DisplayNotFound,
    DuplicateSerialNumber,
    JsonSerError(serde_json_core::ser::Error),
    JsonDeError(serde_json_core::de::Error),
    // BleError(bluest::Error),
    BtleError(btleplug::Error),
    BleAdapterNotFound,
    MissingService,
    MissingCharacteristic,
    MissingNotification,
    CapacityError,
    UuidError(uuid::Error),
    StringSerError(StringSerializerError),
    StringDeError(StringDeserializerError),
    CryptoFetchError(crypto_fetch::Error),
}

impl From<nusb::Error> for Error {
    fn from(value: nusb::Error) -> Self {
        Error::UsbError(value)
    }
}

impl From<ActiveConfigurationError> for Error {
    fn from(value: ActiveConfigurationError) -> Self {
        Error::ActiveConfigurationError(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IoError(value)
    }
}

impl From<serde_json_core::ser::Error> for Error {
    fn from(value: serde_json_core::ser::Error) -> Self {
        Error::JsonSerError(value)
    }
}

impl From<serde_json_core::de::Error> for Error {
    fn from(value: serde_json_core::de::Error) -> Self {
        Error::JsonDeError(value)
    }
}

impl From<btleplug::Error> for Error {
    fn from(value: btleplug::Error) -> Self {
        Error::BtleError(value)
    }
}

impl From<CapacityError> for Error {
    fn from(value: CapacityError) -> Self {
        Error::CapacityError
    }
}

impl From<uuid::Error> for Error {
    fn from(value: uuid::Error) -> Self {
        Error::UuidError(value)
    }
}

impl From<StringSerializerError> for Error {
    fn from(value: StringSerializerError) -> Self {
        Error::StringSerError(value)
    }
}

impl From<StringDeserializerError> for Error {
    fn from(value: StringDeserializerError) -> Self {
        Error::StringDeError(value)
    }
}

impl From<crypto_fetch::Error> for Error{
    fn from(value: crypto_fetch::Error) -> Self {
        Error::CryptoFetchError(value)
    }
}
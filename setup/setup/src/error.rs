use itertools::ExactlyOneError;
use nusb::ActiveConfigurationError;

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

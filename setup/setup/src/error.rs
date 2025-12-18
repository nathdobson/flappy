use nusb::ActiveConfigurationError;

#[derive(Debug)]
pub enum Error {
    UsbError(nusb::Error),
    ActiveConfigurationError(ActiveConfigurationError),
    IoError(std::io::Error),
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
use heapless::CapacityError;
use protocol::setup::WriteSettingsError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[cfg(feature = "ble")]
    #[error("bluetooth driver error: {0}")]
    BtleplugError(#[from] btleplug::Error),

    #[cfg(feature = "ble")]
    #[error("bluetooth application error: {0}")]
    BleError(#[from] crate::ble::BleError),

    #[cfg(feature = "usb")]
    #[error("usb driver error: {0}")]
    NusbError(#[from] nusb::Error),

    #[cfg(feature = "usb")]
    #[error("usb configuration error: {0}")]
    NusbConfigurationError(#[from] nusb::ActiveConfigurationError),

    #[cfg(feature = "usb")]
    #[error("usb transfer error: {0}")]
    NusbTransferError(#[from] nusb::transfer::TransferError),

    #[cfg(feature = "usb")]
    #[error("picoboot error: {0}")]
    PicobootError(#[from] picoboot::Error),

    #[cfg(feature = "usb")]
    #[error("usb application error: {0}")]
    UsbError(#[from] crate::usb::UsbError),

    #[cfg(feature = "ble")]
    #[error("string serialization error: {0}")]
    StringSerializerError(#[from] serde_string::StringSerializerError),

    #[error("json serialization error: {0}")]
    JsonSerializationError(#[from] serde_json_core::ser::Error),

    #[error("json deserialization error: {0}")]
    JsonDeserializationError(#[from] serde_json_core::de::Error),

    #[error("allocation error")]
    AllocError,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("bad response")]
    BadResponse,

    #[error("error writing settings: {0}")]
    WriteSettingsError(#[from] WriteSettingsError),

    #[error("Device is booted in Picoboot mode, but needs Application mode")]
    NeedsApplication,
    #[error("Device is booted in Application mode, but needs Picoboot mode")]
    NeedsPicoboot,
}

impl From<CapacityError> for Error {
    fn from(_: CapacityError) -> Self {
        Error::AllocError
    }
}

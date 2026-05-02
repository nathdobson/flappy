use heapless::CapacityError;
use protocol::setup::WriteSettingsError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[cfg(feature = "ble")]
    #[error("bluetooth driver error")]
    BtleplugError(#[from] btleplug::Error),

    #[cfg(feature = "ble")]
    #[error("bluetooth application error")]
    BleError(#[from] crate::ble::BleError),

    #[cfg(feature = "usb")]
    #[error("usb driver error")]
    NusbError(#[from] nusb::Error),

    #[cfg(feature = "usb")]
    #[error("usb configuration error")]
    NusbConfigurationError(#[from] nusb::ActiveConfigurationError),

    #[cfg(feature = "usb")]
    #[error("usb transfer error")]
    NusbTransferError(#[from] nusb::transfer::TransferError),

    #[cfg(feature = "usb")]
    #[error("picoboot error")]
    PicobootError(#[from] picoboot::Error),

    #[cfg(feature = "usb")]
    #[error("usb application error")]
    UsbError(#[from] crate::usb::UsbError),

    #[cfg(feature = "ble")]
    #[error("string serialization error")]
    StringSerializerError(#[from] serde_string::StringSerializerError),

    #[error("json serialization error")]
    JsonSerializationError(#[from] serde_json_core::ser::Error),

    #[error("json deserialization error")]
    JsonDeserializationError(#[from] serde_json_core::de::Error),

    #[error("allocation error")]
    AllocError,

    #[error("IO error")]
    IoError(#[from] std::io::Error),

    #[error("bad response")]
    BadResponse,

    #[error("error writing settings")]
    WriteSettingsError(#[from] WriteSettingsError),

    #[error("Device is booted in Picoboot mode, but needs Application mode")]
    NeedsApplication,
    #[error("Device is booted in Application mode, but needs Picoboot mode")]
    NeedsPicoboot,
    #[error("Failed to verify binary written to flash")]
    FlashVerifyError,
}

impl From<CapacityError> for Error {
    fn from(_: CapacityError) -> Self {
        Error::AllocError
    }
}

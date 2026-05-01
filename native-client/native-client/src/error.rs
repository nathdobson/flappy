use itertools::ExactlyOneError;
use serde_json_core::heapless::CapacityError;
use serde_string::{StringDeserializerError, StringSerializerError};
use setup_client::client::ClientTransport;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("feature not enabled: {0}")]
    FeatureNotEnabled(ClientTransport),
    #[error("setup client error")]
    SetupClientError(#[from] setup_client::error::Error),
    #[error("json serialization error")]
    JsonSerializationError(#[from] serde_json_core::ser::Error),
    #[error("json deserialization error")]
    JsonDeserializationError(#[from] serde_json_core::de::Error),
    #[error("IO error")]
    IoError(#[from] io::Error),
    #[error("device not found")]
    DeviceNotFound,
    #[error("Failed to reboot device in picoboot mode")]
    RebootFailed,
    #[cfg(feature = "usb")]
    #[error("picoboot error")]
    PicobootError(#[from] picoboot::Error),
}

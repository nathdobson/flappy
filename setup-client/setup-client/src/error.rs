use itertools::ExactlyOneError;
use serde_json_core::heapless::CapacityError;
use serde_string::{StringDeserializerError, StringSerializerError};
use setup_client_lib::client::ClientTransport;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("feature not enabled: {0}")]
    FeatureNotEnabled(ClientTransport),
    #[error("setup client error: {0}")]
    SetupClientError(#[from] setup_client_lib::error::Error),
    #[error("json serialization error: {0}")]
    JsonSerializationError(#[from] serde_json_core::ser::Error),
    #[error("json deserialization error: {0}")]
    JsonDeserializationError(#[from] serde_json_core::de::Error),
    #[error("io error: {0}")]
    IoError(#[from] io::Error),
    #[error("device not found")]
    DeviceNotFound,
    #[error("Failed to reboot device in picoboot mode")]
    RebootFailed,
    #[cfg(feature = "usb")]
    #[error("picoboot error: {0}")]
    PicobootError(#[from] picoboot::Error),
}

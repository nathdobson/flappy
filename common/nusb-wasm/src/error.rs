use thiserror::Error;
use crate::platform;

#[derive(Error, Debug)]
pub enum Error {
    #[error("USB missing descriptor")]
    MissingDescriptor,

    #[error("USB missing data")]
    MissingData,

    #[error("USB missing data")]
    TruncatedWrite,

    #[cfg(not(target_family = "wasm"))]
    #[error("USB error: {0}")]
    NusbError(#[from] nusb::Error),

    #[cfg(not(target_family = "wasm"))]
    #[error("USB configuration error: {0}")]
    ActiveConfigurationError(#[from] nusb::ActiveConfigurationError),

    #[cfg(not(target_family = "wasm"))]
    #[error("Transfer error {0}")]
    TransferError(#[from] nusb::transfer::TransferError),

    #[cfg(target_family = "wasm")]
    #[error("{0}")]
    JsError(js_error::JsValueError),

    #[cfg(target_family = "wasm")]
    #[error("Transfer error {0}")]
    TransferError(#[from] platform::error::TransferError),
}

#[cfg(target_family = "wasm")]
impl From<wasm_bindgen::JsValue> for Error {
    fn from(err: wasm_bindgen::JsValue) -> Error {
        Error::JsError(err.into())
    }
}

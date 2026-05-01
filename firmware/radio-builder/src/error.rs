use embassy_executor::SpawnError;
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[cfg(feature = "ble")]
    #[error("trouble host error")]
    TroubleHostError(#[from] trouble_host::Error),

    #[cfg(feature = "ble")]
    #[error("ble error")]
    BleError(#[from] cyw43::bluetooth::Error),

    #[cfg(feature = "ble")]
    #[error("gatt config error {0}")]
    GattConfigError(&'static str),

    #[cfg(feature = "ble")]
    #[error("gatt disconnect")]
    GattDisconnect(#[from] bt_hci::param::Error),

    #[cfg(feature = "ble")]
    #[error("concurrent requests")]
    ConcurrentRequests,

    #[cfg(feature = "ble")]
    #[error("from gatt error")]
    FromGattError(#[from] trouble_host::types::gatt_traits::FromGattError),

    #[cfg(feature = "ble")]
    #[error("request too large")]
    RequestTooLarge,

    #[cfg(feature = "ble")]
    #[error("status too long")]
    StatusTooLarge,

    #[error("spawn error")]
    SpawnError(#[from] SpawnError),

    #[cfg(feature = "ble")]
    #[error("serde json deserializer error")]
    SerdeDeError(#[from] serde_json_core::de::Error),

    #[cfg(feature = "ble")]
    #[error("serde json serializer error")]
    SerdeSerError(#[from] serde_json_core::ser::Error),
}

#[cfg(feature = "ble")]
impl From<trouble_host::codec::Error> for Error {
    fn from(error: trouble_host::codec::Error) -> Self {
        Error::TroubleHostError(error.into())
    }
}

#[cfg(feature = "ble")]
impl<T> From<trouble_host::BleHostError<T>> for Error
where
    Error: From<T>,
{
    fn from(value: trouble_host::BleHostError<T>) -> Self {
        match value {
            trouble_host::BleHostError::Controller(x) => x.into(),
            trouble_host::BleHostError::BleHost(x) => x.into(),
        }
    }
}

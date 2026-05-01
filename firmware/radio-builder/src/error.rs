use embassy_executor::SpawnError;
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[cfg(feature = "ble")]
    #[error("trouble host error {0:?}")]
    TroubleHostError(#[from] trouble_host::Error),
    #[cfg(feature = "ble")]
    #[error("trouble host codec error {0}")]
    TroubleHostCodecError(#[from] trouble_host::codec::Error),
    #[cfg(feature = "ble")]
    #[error("trouble host host error {0}")]
    TroubleHostHostError(#[from] trouble_host::BleHostError<cyw43::bluetooth::Error>),
    #[cfg(feature = "ble")]
    #[error("gatt config error {0}")]
    GattConfigError(&'static str),
    #[cfg(feature = "ble")]
    #[error("gatt disconnect {0:?}")]
    GattDisconnect(bt_hci::param::Status),
    #[cfg(feature = "ble")]
    #[error("concurrent requests")]
    ConcurrentRequests,
    #[cfg(feature = "ble")]
    #[error("from gatt error {0}")]
    FromGattError(#[from] trouble_host::types::gatt_traits::FromGattError),
    #[cfg(feature = "ble")]
    #[error("request too large")]
    RequestTooLarge,
    #[cfg(feature = "ble")]
    #[error("status too long")]
    StatusTooLarge,

    #[error("spawn error {0}")]
    SpawnError(#[from] SpawnError),

    #[cfg(feature = "ble")]
    #[error("serde json deserializer error {0}")]
    SerdeDeError(#[from] serde_json_core::de::Error),

    #[cfg(feature = "ble")]
    #[error("serde json serializer error {0}")]
    SerdeSerError(#[from] serde_json_core::ser::Error),
}

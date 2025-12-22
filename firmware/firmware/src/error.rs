use core::fmt;
use core::fmt::{Display, Formatter};
use core::num::ParseIntError;
use embassy_executor::SpawnError;
use embassy_time::TimeoutError;
use embassy_usb::driver::EndpointError;
use heapless::CapacityError;
use trouble_host::types::gatt_traits::FromGattError;

#[derive(Debug)]
pub enum Error {
    SpawnError(SpawnError),
    #[cfg(feature = "radio")]
    UuidError(uuid::Error),
    #[cfg(feature = "radio")]
    TroubleError(trouble_host::Error),
    #[cfg(feature = "radio")]
    TroubleCodecError(trouble_host::codec::Error),
    #[cfg(feature = "radio")]
    BleHostError(trouble_host::BleHostError<cyw43::bluetooth::Error>),
    StrError(&'static str),
    FlashError(embassy_rp::flash::Error),
    #[cfg(feature = "serde")]
    JsonDeError(serde_json_core::de::Error),
    #[cfg(feature = "serde")]
    JsonSerError(serde_json_core::ser::Error),
    #[cfg(feature = "radio")]
    DnsError(embassy_net::dns::Error),
    #[cfg(feature = "radio")]
    MqttError(mqtt_core::error::ProtocolError),
    #[cfg(feature = "radio")]
    TlsError(embedded_tls::TlsError),
    #[cfg(feature = "radio")]
    ConnectError(embassy_net::tcp::ConnectError),
    SpiError(embassy_rp::spi::Error),
    FmtError,
    #[cfg(feature = "radio")]
    DeadlineExceeded,
    #[cfg(feature = "radio")]
    Disconnected(mqtt_core::protocol::ReasonCode),
    CapacityError,
    ParseIntError,
    EndpointError(EndpointError),
    NotEnoughReceivers,
    #[cfg(feature = "radio")]
    FromGattError(FromGattError),
}

pub type Result<T> = core::result::Result<T, Error>;

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::SpawnError(error) => write!(f, "Spawn error: {}", error),
            #[cfg(feature = "radio")]
            Error::UuidError(error) => write!(f, "Uuid error: {}", error),
            #[cfg(feature = "radio")]
            Error::TroubleError(error) => write!(f, "Trouble error: {:?}", error),
            #[cfg(feature = "radio")]
            Error::TroubleCodecError(error) => write!(f, "Trouble codec error: {:?}", error),
            #[cfg(feature = "radio")]
            Error::BleHostError(error) => write!(f, "Ble host error: {:?}", error),
            Error::StrError(error) => write!(f, "Str error: {}", error),
            Error::FlashError(error) => write!(f, "Flash error: {:?}", error),
            #[cfg(feature = "serde")]
            Error::JsonDeError(error) => write!(f, "Json deserialize error: {}", error),
            #[cfg(feature = "serde")]
            Error::JsonSerError(error) => write!(f, "Json serialize error: {}", error),
            #[cfg(feature = "radio")]
            Error::DnsError(error) => write!(f, "DNS error: {:?}", error),
            #[cfg(feature = "radio")]
            Error::MqttError(error) => write!(f, "MQTT error: {}", error),
            #[cfg(feature = "radio")]
            Error::TlsError(error) => write!(f, "TLS error: {:?}", error),
            #[cfg(feature = "radio")]
            Error::ConnectError(error) => write!(f, "Connect error: {:?}", error),
            Error::SpiError(error) => write!(f, "SPI error: {:?}", error),
            Error::FmtError => write!(f, "Format error"),
            #[cfg(feature = "radio")]
            Error::DeadlineExceeded => write!(f, "Deadline exceeded"),
            #[cfg(feature = "radio")]
            Error::Disconnected(r) => write!(f, "Server disconnected {}", r),
            Error::CapacityError => write!(f, "Capacity error"),
            Error::ParseIntError => write!(f, "Parse int error"),
            Error::EndpointError(e) => write!(f, "Endpoint error: {:?}", e),
            Error::NotEnoughReceivers => write!(f, "Not enough receivers"),
            #[cfg(feature = "radio")]
            Error::FromGattError(e) => write!(f, "Error converting GATT data: {:?}", e),
        }
    }
}

impl From<SpawnError> for Error {
    fn from(error: SpawnError) -> Self {
        Error::SpawnError(error)
    }
}

#[cfg(feature = "radio")]
impl From<uuid::Error> for Error {
    fn from(error: uuid::Error) -> Self {
        Error::UuidError(error)
    }
}

#[cfg(feature = "radio")]
impl From<trouble_host::Error> for Error {
    fn from(error: trouble_host::Error) -> Self {
        Error::TroubleError(error)
    }
}

#[cfg(feature = "radio")]
impl From<trouble_host::codec::Error> for Error {
    fn from(error: trouble_host::codec::Error) -> Self {
        Error::TroubleCodecError(error)
    }
}

#[cfg(feature = "radio")]
impl From<trouble_host::BleHostError<cyw43::bluetooth::Error>> for Error {
    fn from(error: trouble_host::BleHostError<cyw43::bluetooth::Error>) -> Self {
        Error::BleHostError(error)
    }
}

impl From<&'static str> for Error {
    fn from(value: &'static str) -> Self {
        Self::StrError(value)
    }
}

impl From<embassy_rp::flash::Error> for Error {
    fn from(error: embassy_rp::flash::Error) -> Self {
        Error::FlashError(error)
    }
}

#[cfg(feature = "serde")]
impl From<serde_json_core::de::Error> for Error {
    fn from(value: serde_json_core::de::Error) -> Self {
        Error::JsonDeError(value)
    }
}

#[cfg(feature = "serde")]
impl From<serde_json_core::ser::Error> for Error {
    fn from(value: serde_json_core::ser::Error) -> Self {
        Error::JsonSerError(value)
    }
}

#[cfg(feature = "radio")]
impl From<embassy_net::dns::Error> for Error {
    fn from(value: embassy_net::dns::Error) -> Self {
        Error::DnsError(value)
    }
}

#[cfg(feature = "radio")]
impl From<embedded_tls::TlsError> for Error {
    fn from(error: embedded_tls::TlsError) -> Self {
        Error::TlsError(error)
    }
}

#[cfg(feature = "radio")]
impl From<embassy_net::tcp::ConnectError> for Error {
    fn from(error: embassy_net::tcp::ConnectError) -> Self {
        Error::ConnectError(error)
    }
}

impl From<embassy_rp::spi::Error> for Error {
    fn from(value: embassy_rp::spi::Error) -> Self {
        Error::SpiError(value)
    }
}

impl From<fmt::Error> for Error {
    fn from(value: fmt::Error) -> Self {
        Error::FmtError
    }
}

#[cfg(feature = "radio")]
impl<E> From<mqtt_client::error::Error<E>> for Error
where
    Error: From<E>,
{
    fn from(value: mqtt_client::error::Error<E>) -> Self {
        match value {
            mqtt_client::error::Error::NetworkError(e) => Error::from(e),
            mqtt_client::error::Error::ProtocolError(e) => Error::MqttError(e),
        }
    }
}

#[cfg(feature = "radio")]
impl From<mqtt_core::error::ProtocolError> for Error {
    fn from(value: mqtt_core::error::ProtocolError) -> Self {
        Error::MqttError(value)
    }
}

impl From<CapacityError> for Error {
    fn from(value: CapacityError) -> Self {
        Error::CapacityError
    }
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Error::ParseIntError
    }
}

impl From<EndpointError> for Error {
    fn from(value: EndpointError) -> Self {
        Error::EndpointError(value)
    }
}

impl From<TimeoutError> for Error {
    fn from(value: TimeoutError) -> Self {
        Error::DeadlineExceeded
    }
}

#[cfg(feature = "radio")]
impl From<FromGattError> for Error {
    fn from(value: FromGattError) -> Self {
        Error::FromGattError(value)
    }
}

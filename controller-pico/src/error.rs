use core::fmt;
use core::fmt::{Display, Formatter};
use embassy_executor::SpawnError;
use embassy_net::tcp::ConnectError;
use embedded_tls::TlsError;
use rust_mqtt::packet::v5::reason_codes::ReasonCode;
use trouble_host::{BleHostError, codec};

#[derive(Debug)]
pub enum Error {
    SpawnError(SpawnError),
    UuidError(uuid::Error),
    TroubleError(trouble_host::Error),
    TroubleCodecError(codec::Error),
    BleHostError(BleHostError<cyw43::bluetooth::Error>),
    StrError(&'static str),
    FlashError(embassy_rp::flash::Error),
    JsonDeError(serde_json_core::de::Error),
    JsonSerError(serde_json_core::ser::Error),
    DnsError(embassy_net::dns::Error),
    MqttError(ReasonCode),
    TlsError(TlsError),
    ConnectError(ConnectError),
    SpiError(embassy_rp::spi::Error),
    FmtError,
}

pub type Result<T> = core::result::Result<T, Error>;

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::SpawnError(error) => write!(f, "Spawn error: {}", error),
            Error::UuidError(error) => write!(f, "Uuid error: {}", error),
            Error::TroubleError(error) => write!(f, "Trouble error: {:?}", error),
            Error::TroubleCodecError(error) => write!(f, "Trouble codec error: {:?}", error),
            Error::BleHostError(error) => write!(f, "Ble host error: {:?}", error),
            Error::StrError(error) => write!(f, "Str error: {}", error),
            Error::FlashError(error) => write!(f, "Flash error: {:?}", error),
            Error::JsonDeError(error) => write!(f, "Json deserialize error: {}", error),
            Error::JsonSerError(error) => write!(f, "Json serialize error: {}", error),
            Error::DnsError(error) => write!(f, "DNS error: {:?}", error),
            Error::MqttError(error) => write!(f, "MQTT error: {}", error),
            Error::TlsError(error) => write!(f, "TLS error: {:?}", error),
            Error::ConnectError(error) => write!(f, "Connect error: {:?}", error),
            Error::SpiError(error) => write!(f, "SPI error: {:?}", error),
            Error::FmtError => write!(f, "Format error"),
        }
    }
}

impl From<SpawnError> for Error {
    fn from(error: SpawnError) -> Self {
        Error::SpawnError(error)
    }
}

impl From<uuid::Error> for Error {
    fn from(error: uuid::Error) -> Self {
        Error::UuidError(error)
    }
}

impl From<trouble_host::Error> for Error {
    fn from(error: trouble_host::Error) -> Self {
        Error::TroubleError(error)
    }
}

impl From<codec::Error> for Error {
    fn from(error: codec::Error) -> Self {
        Error::TroubleCodecError(error)
    }
}

impl From<BleHostError<cyw43::bluetooth::Error>> for Error {
    fn from(error: BleHostError<cyw43::bluetooth::Error>) -> Self {
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

impl From<serde_json_core::de::Error> for Error {
    fn from(value: serde_json_core::de::Error) -> Self {
        Error::JsonDeError(value)
    }
}

impl From<serde_json_core::ser::Error> for Error {
    fn from(value: serde_json_core::ser::Error) -> Self {
        Error::JsonSerError(value)
    }
}

impl From<embassy_net::dns::Error> for Error {
    fn from(value: embassy_net::dns::Error) -> Self {
        Error::DnsError(value)
    }
}

impl From<ReasonCode> for Error {
    fn from(value: ReasonCode) -> Self {
        Error::MqttError(value)
    }
}

impl From<TlsError> for Error {
    fn from(error: TlsError) -> Self {
        Error::TlsError(error)
    }
}

impl From<ConnectError> for Error {
    fn from(error: ConnectError) -> Self {
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

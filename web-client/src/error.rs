use heapless::CapacityError;
use io_adapters::tokio::TokioErrorAdapter;
use mqtt_core::error::ProtocolError;
use mqtt_core::protocol::ReasonCode;
use protocol::setup::WriteSettingsError;
use std::any::Any;
use std::fmt::{Display, Formatter};
use tokio::sync::mpsc::error::{SendError, TrySendError};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{DomException, UsbTransferStatus};

#[derive(Debug)]
pub enum Error {
    JsError(JsValue),
    UrlError(url::ParseError),
    QueryStringError(serde_qs::Error),
    WsError(ws_stream_wasm::WsErr),
    IoError(std::io::Error),
    DeadlineExceeded,
    MqttError(ProtocolError),
    TypeError,
    JsonSerError(serde_json_core::ser::Error),
    CapacityError,
    TrySendError,
    Disconnect(ReasonCode),
    SendError,
    UnexpectedEof,
    RecvError,
    Panic(Box<dyn Any + Send>),
    ChannelClosed,
    CannotFindElement,
    BluetoothNotSupported,
    SerdeDeError(serde_json_core::de::Error),
    MissingStatusValue,
    BadResponse,
    WriteSettingsError(WriteSettingsError),
    ExpectedSingleFile,
    NotConnected,
    UsbConfigurationNotFound,
    UsbMissingInterface,
    UsbMissingEndpoint,
    UsbTransferError(UsbTransferStatus),
    UsbMissingData,
    ParseIntError(std::num::ParseIntError),
    NotApplicationMode,
    Picoboot(picoboot::Error),
}

impl From<JsValue> for Error {
    fn from(value: JsValue) -> Self {
        Error::JsError(value)
    }
}

impl From<url::ParseError> for Error {
    fn from(value: url::ParseError) -> Self {
        Error::UrlError(value)
    }
}

impl From<serde_qs::Error> for Error {
    fn from(value: serde_qs::Error) -> Self {
        Error::QueryStringError(value)
    }
}

impl From<ws_stream_wasm::WsErr> for Error {
    fn from(value: ws_stream_wasm::WsErr) -> Self {
        Error::WsError(value)
    }
}

impl<T> From<mqtt_client::error::Error<T>> for Error
where
    Error: From<T>,
{
    fn from(value: mqtt_client::error::Error<T>) -> Self {
        match value {
            mqtt_client::error::Error::NetworkError(e) => Error::from(e),
            mqtt_client::error::Error::ProtocolError(e) => Error::MqttError(e),
        }
    }
}

impl From<TokioErrorAdapter> for Error {
    fn from(e: TokioErrorAdapter) -> Self {
        Error::IoError(e.0)
    }
}

impl From<serde_json_core::ser::Error> for Error {
    fn from(value: serde_json_core::ser::Error) -> Self {
        Error::JsonSerError(value)
    }
}

impl From<CapacityError> for Error {
    fn from(value: CapacityError) -> Self {
        Error::CapacityError
    }
}

impl<T> From<TrySendError<T>> for Error {
    fn from(value: TrySendError<T>) -> Self {
        Error::TrySendError
    }
}

impl<T> From<SendError<T>> for Error {
    fn from(value: SendError<T>) -> Self {
        Error::SendError
    }
}

impl From<Box<dyn Any + Send>> for Error {
    fn from(x: Box<dyn Any + Send + 'static>) -> Self {
        Error::Panic(x)
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for Error {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        Error::RecvError
    }
}

impl From<serde_json_core::de::Error> for Error {
    fn from(value: serde_json_core::de::Error) -> Self {
        Error::SerdeDeError(value)
    }
}

impl From<WriteSettingsError> for Error {
    fn from(value: WriteSettingsError) -> Self {
        Error::WriteSettingsError(value)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(value: std::num::ParseIntError) -> Self {
        Error::ParseIntError(value)
    }
}

impl From<picoboot::Error> for Error{
    fn from(value: picoboot::Error) -> Self {
        Error::Picoboot(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::JsError(e) => {
                if let Ok(e) = e.clone().dyn_into::<DomException>() {
                    write!(f, "{} ({})", e.message(), e.name())?;
                } else {
                    write!(f, "JsError: {:?}", e)?;
                }
                Ok(())
            }
            Error::UrlError(x) => write!(f, "{}", x),
            Error::QueryStringError(x) => write!(f, "{}", x),
            Error::WsError(x) => write!(f, "{}", x),
            Error::IoError(x) => write!(f, "{}", x),
            Error::DeadlineExceeded => write!(f, "deadline exceeded"),
            Error::MqttError(x) => write!(f, "{}", x),
            Error::TypeError => write!(f, "type error"),
            Error::JsonSerError(x) => write!(f, "{}", x),
            Error::CapacityError => write!(f, "capacity error"),
            Error::TrySendError => write!(f, "failed to send internal message"),
            Error::Disconnect(x) => write!(f, "disconnected: {}", x),
            Error::SendError => write!(f, "failed to send internal message"),
            Error::UnexpectedEof => write!(f, "unexpected end of message"),
            Error::RecvError => write!(f, "failed to receive internal message"),
            Error::Panic(x) => {
                let x: &(dyn Any + Send) = &**x;
                if let Some(x) = x.downcast_ref::<String>() {
                    write!(f, "panic {}", x)
                } else if let Some(x) = x.downcast_ref::<&str>() {
                    write!(f, "panic {}", x)
                } else {
                    write!(f, "panic")
                }
            }
            Error::ChannelClosed => write!(f, "channel closed"),
            Error::CannotFindElement => write!(f, "cannot find element"),
            Error::BluetoothNotSupported => write!(f, "Bluetooth not supported"),
            Error::SerdeDeError(e) => write!(f, "Deserialization error: {}", e),
            Error::MissingStatusValue => write!(f, "missing status value"),
            Error::BadResponse => write!(f, "bad response"),
            Error::WriteSettingsError(e) => write!(f, "Write settings error: {}", e),
            Error::ExpectedSingleFile => write!(f, "expected single file"),
            Error::NotConnected => write!(f, "not connected"),
            Error::UsbConfigurationNotFound => write!(f, "USB configuration not found"),
            Error::UsbMissingInterface => write!(f, "USB missing interface"),
            Error::UsbMissingEndpoint => write!(f, "USB missing endpoint"),
            Error::UsbTransferError(x) => write!(f, "USB transfer error: {:?}", x),
            Error::UsbMissingData => write!(f, "USB missing data"),
            Error::ParseIntError(e) => write!(f, "parse int error: {}", e),
            Error::NotApplicationMode => write!(f, "Device not in application mode. Restart device with no buttons pressed."),
            Error::Picoboot(e) => write!(f, "{}", e),
        }
    }
}

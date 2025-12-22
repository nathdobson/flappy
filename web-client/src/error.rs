use heapless::CapacityError;
use io_adapters::tokio::TokioErrorAdapter;
use mqtt_core::protocol::ReasonCode;
use tokio::sync::mpsc::error::{SendError, TrySendError};
use wasm_bindgen::JsValue;
use mqtt_core::error::ProtocolError;

#[derive(Debug)]
pub enum Error {
    JsError(JsValue),
    NoneError,
    UrlError(url::ParseError),
    QueryStringError(serde_qs::Error),
    WsError(ws_stream_wasm::WsErr),
    IoError(std::io::Error),
    DeadlineExceeded,
    MqttError(ProtocolError),
    TypeError,
    JsonSerError(serde_json_core::ser::Error),
    CapacityError(CapacityError),
    TrySendError,
    Disconnect(ReasonCode),
    SendError,
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
        Error::CapacityError(value)
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

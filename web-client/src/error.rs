use heapless::CapacityError;
use io_adapters::tokio::TokioErrorAdapter;
use mqtt_core::error::ProtocolError;
use mqtt_core::protocol::ReasonCode;
use protocol::setup::WriteSettingsError;
use std::any::Any;
use std::fmt::{Debug, Display, Formatter};
use std::str::ParseBoolError;
use thiserror::Error;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::DomException;

#[derive(Debug, Clone)]
pub struct JsErrorAdapter(JsValue);

impl From<JsValue> for JsErrorAdapter {
    fn from(value: JsValue) -> Self {
        JsErrorAdapter(value)
    }
}

impl Into<JsValue> for JsErrorAdapter {
    fn into(self) -> JsValue {
        self.0
    }
}

impl Display for JsErrorAdapter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Ok(e) = self.0.clone().dyn_into::<DomException>() {
            write!(f, "{} ({})", e.message(), e.name())?;
        } else {
            write!(f, "JsError: {:?}", self)?;
        }
        Ok(())
    }
}

impl core::error::Error for JsErrorAdapter {}

pub struct PanicError(Box<dyn Any + Send + 'static>);
impl From<Box<dyn Any + Send + 'static>> for PanicError {
    fn from(value: Box<dyn Any + Send + 'static>) -> Self {
        PanicError(value)
    }
}

impl From<PanicError> for Box<dyn Any + Send + 'static> {
    fn from(value: PanicError) -> Self {
        value.0
    }
}

impl Debug for PanicError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_tuple("PanicError");
        if let Some(x) = (*self.0).downcast_ref::<String>() {
            f.field(x).finish()
        } else if let Some(x) = (*self.0).downcast_ref::<&str>() {
            f.field(x).finish()
        } else {
            return f.finish_non_exhaustive();
        }
    }
}

impl Display for PanicError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(x) = (*self.0).downcast_ref::<String>() {
            write!(f, "panic: {}", x)
        } else if let Some(x) = (*self.0).downcast_ref::<&str>() {
            write!(f, "panic: {}", x)
        } else {
            write!(f, "panic: ?")
        }
    }
}

impl core::error::Error for PanicError {}

#[derive(Debug, Error)]
pub enum Error {
    #[error("javascript error")]
    JsError(#[from] JsErrorAdapter),
    #[error("URL parse error")]
    UrlError(#[from] url::ParseError),
    #[error("query string error")]
    QueryStringError(#[from] serde_qs::Error),
    #[error("websocket error")]
    WsError(#[from] ws_stream_wasm::WsErr),
    #[error("IO error")]
    IoError(#[from] std::io::Error),
    #[error("MQTT error")]
    MqttError(#[from] ProtocolError),
    #[error("type cast error")]
    TypeError,
    #[error("serde json serialization error")]
    JsonSerError(#[from] serde_json_core::ser::Error),
    #[error("serde json deserialization error")]
    SerdeDeError(#[from] serde_json_core::de::Error),
    #[error("capacity error")]
    CapacityError,
    #[error("MQTT disconnect")]
    Disconnect(#[from] ReasonCode),
    #[error("send error")]
    SendError,
    #[error("unexpected EOF")]
    UnexpectedEof,
    #[error("receive error")]
    RecvError(#[from] tokio::sync::oneshot::error::RecvError),
    #[error("panic")]
    Panic(#[from] PanicError),
    #[error("channel closed")]
    ChannelClosed,
    #[error("cannot find element")]
    CannotFindElement,
    #[error("Bluetooth not supported by browser")]
    BluetoothNotSupported,
    #[error("error writing settings")]
    WriteSettingsError(#[from] WriteSettingsError),
    #[error("not connected")]
    NotConnected,
    #[error("error parsing integer")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("picoboot error")]
    Picoboot(#[from] picoboot::Error),
    #[error("BLE error")]
    BtleplugError(#[from] btleplug::Error),
    #[error("setup error")]
    SetupClientError(#[from] setup_client::error::Error),
    #[error("usb error")]
    UsbError(#[from] nusb::Error),
    #[error("microcontroller not in picoboot mode")]
    NotPicobootMode,
    #[error("USB not supported by browser")]
    UsbNotSupported,
    #[error("error parsing boolean")]
    ParseBoolError(#[from] ParseBoolError),
}

impl From<CapacityError> for Error {
    fn from(_: CapacityError) -> Self {
        Error::CapacityError
    }
}

impl From<Box<dyn Any + Send + 'static>> for Error {
    fn from(value: Box<dyn Any + Send + 'static>) -> Self {
        Error::Panic(value.into())
    }
}

impl From<JsValue> for Error {
    fn from(value: JsValue) -> Self {
        Error::JsError(value.into())
    }
}

impl<W, R> From<mqtt_client::Error<W, R>> for Error
where
    Error: From<W>,
    Error: From<R>,
{
    fn from(value: mqtt_client::Error<W, R>) -> Self {
        match value {
            mqtt_client::Error::WriteError(e) => e.into(),
            mqtt_client::Error::ReadError(e) => e.into(),
            mqtt_client::Error::ProtocolError(e) => e.into(),
        }
    }
}

impl From<TokioErrorAdapter> for Error {
    fn from(e: TokioErrorAdapter) -> Self {
        Error::IoError(e.0)
    }
}

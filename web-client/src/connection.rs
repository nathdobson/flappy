use crate::ble_connection::BleConnection;
use crate::error::Error;
use crate::status::Status;
use crate::usb_connection::UsbConnection;
use protocol::setup::{AppSettings, AppStatus, DeviceInfo, SetupRequest, SetupResponse};
use std::rc::Rc;

#[derive(Clone)]
pub enum Connection {
    UsbConnection(Rc<UsbConnection>),
    BleConnection(Rc<BleConnection>),
}

pub enum ConnectionType {
    Usb,
    Ble,
}

impl Connection {
    pub async fn new(typ: ConnectionType, connect_status: Rc<Status>) -> Result<Connection, Error> {
        match typ {
            ConnectionType::Usb => Ok(Connection::UsbConnection(
                UsbConnection::new(connect_status).await?,
            )),
            ConnectionType::Ble => Ok(Connection::BleConnection(
                BleConnection::new(connect_status).await?,
            )),
        }
    }
    pub async fn device_info(&self) -> Result<DeviceInfo, Error> {
        match self.invoke(SetupRequest::DeviceInfo).await? {
            SetupResponse::DeviceInfo(device_info) => Ok(device_info),
            _ => Err(Error::BadResponse),
        }
    }
    pub async fn touch_app_status(&self) -> Result<(), Error> {
        match self.invoke(SetupRequest::TouchAppStatus).await? {
            SetupResponse::TouchAppStatus => Ok(()),
            _ => Err(Error::BadResponse),
        }
    }
    pub async fn read_settings(&self) -> Result<AppSettings, Error> {
        match self.invoke(SetupRequest::ReadSettings).await? {
            SetupResponse::ReadSettings(settings) => Ok(settings),
            _ => Err(Error::BadResponse),
        }
    }
    pub async fn write_settings(&self, settings: AppSettings) -> Result<(), Error> {
        match self.invoke(SetupRequest::WriteSettings(settings)).await? {
            SetupResponse::WriteSettings(settings) => Ok(settings?),
            _ => Err(Error::BadResponse),
        }
    }
    
    async fn invoke(&self, request: SetupRequest) -> Result<SetupResponse, Error> {
        match self {
            Connection::UsbConnection(connection) => connection.invoke(request).await,
            Connection::BleConnection(connection) => connection.invoke(request).await,
        }
    }
    pub async fn next_status(&self) -> Result<AppStatus, Error> {
        match self {
            Connection::UsbConnection(connection) => connection.next_status().await,
            Connection::BleConnection(connection) => connection.next_status().await,
        }
    }
}

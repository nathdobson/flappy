use crate::error::Error;
use protocol::setup::{
    AppSettings, AppStatus, DeviceInfo, MAX_SETUP_MESSAGE_SIZE, SetupRequest, SetupResponse,
    WriteAppSettings,
};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum ClientTransport {
    Usb,
    Ble,
}

impl Display for ClientTransport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientTransport::Usb => write!(f, "usb"),
            ClientTransport::Ble => write!(f, "ble"),
        }
    }
}

pub enum Client {
    #[cfg(feature = "ble")]
    BleClient(crate::ble::BleClient),
    #[cfg(feature = "usb")]
    UsbClient(crate::usb::UsbClient),
}

impl Client {
    async fn invoke_raw(&self, req: &[u8]) -> Result<Vec<u8>, Error> {
        match self {
            #[cfg(feature = "ble")]
            Client::BleClient(client) => client.invoke_raw(req).await,
            #[cfg(feature = "usb")]
            Client::UsbClient(client) => client.invoke_raw(req).await,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
    async fn invoke(&self, req: &SetupRequest) -> Result<SetupResponse, Error> {
        let response = self
            .invoke_raw(&serde_json_core::to_vec::<_, MAX_SETUP_MESSAGE_SIZE>(req)?.to_vec())
            .await?;
        let mut tmp = [0u8; MAX_SETUP_MESSAGE_SIZE];
        Ok(serde_json_core::from_slice_escaped::<SetupResponse>(&response, &mut tmp)?.0)
    }
    async fn receive_status_raw(&self) -> Result<Vec<u8>, Error> {
        match self {
            #[cfg(feature = "ble")]
            Client::BleClient(client) => client.receive_status_raw().await,
            #[cfg(feature = "usb")]
            Client::UsbClient(client) => client.receive_status_raw().await,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
    pub async fn receive_status(&self) -> Result<AppStatus, Error> {
        let response = self.receive_status_raw().await?;
        let mut tmp = [0u8; MAX_SETUP_MESSAGE_SIZE];
        Ok(serde_json_core::from_slice_escaped::<AppStatus>(&response, &mut tmp)?.0)
    }
    pub async fn device_info(&self) -> Result<DeviceInfo, Error> {
        match self.invoke(&SetupRequest::DeviceInfo).await? {
            SetupResponse::DeviceInfo(device_info) => Ok(device_info),
            _ => Err(Error::BadResponse),
        }
    }
    pub async fn touch_app_status(&self) -> Result<(), Error> {
        match self.invoke(&SetupRequest::TouchAppStatus).await? {
            SetupResponse::TouchAppStatus => Ok(()),
            _ => Err(Error::BadResponse),
        }
    }
    pub async fn read_settings(&self) -> Result<AppSettings, Error> {
        match self.invoke(&SetupRequest::ReadSettings).await? {
            SetupResponse::ReadSettings(settings) => Ok(settings),
            _ => Err(Error::BadResponse),
        }
    }
    pub async fn write_settings(&self, settings: WriteAppSettings) -> Result<(), Error> {
        match self.invoke(&SetupRequest::WriteSettings(settings)).await? {
            SetupResponse::WriteSettings(settings) => Ok(settings?),
            _ => Err(Error::BadResponse),
        }
    }
    pub async fn ping(&self) -> Result<(), Error> {
        match self.invoke(&SetupRequest::Ping).await? {
            SetupResponse::Pong => Ok(()),
            _ => Err(Error::BadResponse),
        }
    }
}

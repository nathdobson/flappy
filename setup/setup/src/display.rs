use crate::error::Error;
use nusb::io::{EndpointRead, EndpointWrite};
use nusb::transfer::{Bulk, Direction, In, Out};
use nusb::{Device, DeviceInfo, Endpoint, list_devices};
use proto::setup::{
    AppStatus, CUSTOM_CLASS_ID, CUSTOM_SUBCLASS_ID, MAX_SETUP_MESSAGE_SIZE, SetupRequest,
    SetupResponse,
};
use std::io::Read;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt};

#[derive(Debug)]
pub struct DisplayInfo {
    device: DeviceInfo,
}

pub struct DisplaySetup {
    request: EndpointWrite<Bulk>,
    response: EndpointRead<Bulk>,
}

pub struct DisplayStatus {
    status: EndpointRead<Bulk>,
}

const BUFFER_SIZE: usize = 64;

impl DisplayInfo {
    pub async fn list() -> Result<Vec<DisplayInfo>, Error> {
        Ok(list_devices()
            .await?
            .filter(|device| {
                device.vendor_id() == proto::setup::VENDOR_ID
                    && device.product_id() == proto::setup::PRODUCT_ID
            })
            .map(|device| DisplayInfo { device })
            .collect())
    }
    pub fn serial_number(&self) -> Option<&str> {
        self.device.serial_number()
    }
    pub async fn connect(&self) -> Result<(DisplaySetup, DisplayStatus), Error> {
        let dev = self.device.open().await?;
        let config = dev.active_configuration()?;
        for int in config.interfaces() {
            if int.first_alt_setting().class() == CUSTOM_CLASS_ID {
                let int = dev.claim_interface(int.interface_number()).await?;
                if let Some(desc) = int.descriptor() {
                    if desc.subclass() == CUSTOM_SUBCLASS_ID {
                        let mut ep = desc.endpoints();
                        let response = int
                            .endpoint::<Bulk, In>(
                                ep.next().ok_or(Error::MissingEndpoint)?.address(),
                            )?
                            .reader(BUFFER_SIZE);
                        let request = int
                            .endpoint::<Bulk, Out>(
                                ep.next().ok_or(Error::MissingEndpoint)?.address(),
                            )?
                            .writer(BUFFER_SIZE);
                        let status = int
                            .endpoint::<Bulk, In>(
                                ep.next().ok_or(Error::MissingEndpoint)?.address(),
                            )?
                            .reader(BUFFER_SIZE);
                        return Ok((DisplaySetup { request, response }, DisplayStatus { status }));
                    }
                }
            }
        }
        Err(Error::MissingInterface)
    }
}

impl DisplaySetup {
    pub async fn invoke(&mut self, req: &SetupRequest) -> Result<SetupResponse, Error> {
        let mut tmp = [0; MAX_SETUP_MESSAGE_SIZE];
        self.request
            .write_all(&serde_json_core::to_vec::<_, MAX_SETUP_MESSAGE_SIZE>(req)?)
            .await?;
        self.request.flush().await?;
        let mut response = vec![];
        self.response
            .until_short_packet()
            .read_to_end(&mut response)?;
        let (resp, _) = serde_json_core::from_slice_escaped::<SetupResponse>(&response, &mut tmp)?;
        Ok(resp)
    }
}

impl DisplayStatus {
    pub async fn receive(&mut self) -> Result<AppStatus, Error> {
        let mut tmp = [0; MAX_SETUP_MESSAGE_SIZE];
        let mut response = vec![];
        assert!(!self.status.fill_buf().await?.is_empty());
        self.status
            .until_short_packet()
            .read_to_end(&mut response)?;
        let (status, _) = serde_json_core::from_slice_escaped::<AppStatus>(&response, &mut tmp)?;
        Ok(status)
    }
}

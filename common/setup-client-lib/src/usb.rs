use crate::error::Error;
use log::info;
use nusb_wasm::{
    ControlOut, ControlType, DeviceInfo, EndpointIn, EndpointOut, Interface, Recipient,
};
use picoboot::Picoboot;
use protocol::setup::{AppStatus, MAX_SETUP_MESSAGE_SIZE, SetupRequest, SetupResponse};
use protocol::usb::{
    APPLICATION_PRODUCT_ID, CUSTOM_CLASS_ID, CUSTOM_SUBCLASS_ID, PICOBOOT_PRODUCT_ID,
    PICOBOOT_RESET_REQUEST_BOOTSEL, PICOBOOT_SUBCLASS_ID, VENDOR_ID,
};
use std::time::Duration;
use thiserror::Error;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct UsbClientBuilder {
    device: DeviceInfo,
}

struct RequestResponse {
    request: EndpointOut,
    response: EndpointIn,
}

pub struct UsbClient {
    request_response: Mutex<RequestResponse>,
    status: Mutex<EndpointIn>,
    reset_int: Mutex<Interface>,
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Debug, Hash, Copy, Clone)]
pub enum BootSelect {
    Application,
    Picoboot,
}

#[derive(Debug, Error)]
pub enum UsbError {
    #[error("USB device has missing endpoint")]
    MissingEndpoint,
    #[error("USB device has missing interface")]
    MissingInterface,
    #[error("USB device has missing descriptor")]
    MissingDescriptor,
}

const BUFFER_SIZE: usize = 64;

impl UsbClientBuilder {
    pub fn from_device_info(device: DeviceInfo) -> Option<UsbClientBuilder> {
        if device.vendor_id() == VENDOR_ID
            && (device.product_id() == APPLICATION_PRODUCT_ID
                || device.product_id() == PICOBOOT_PRODUCT_ID)
        {
            Some(UsbClientBuilder { device })
        } else {
            None
        }
    }
    pub fn serial_number(&self) -> Option<&str> {
        self.device.serial_number()
    }
    pub fn boot_select(&self) -> BootSelect {
        if self.device.product_id() == PICOBOOT_PRODUCT_ID {
            BootSelect::Picoboot
        } else {
            BootSelect::Application
        }
    }
    pub async fn connect(self) -> Result<UsbClient, Error> {
        if self.device.product_id() != APPLICATION_PRODUCT_ID {
            return Err(Error::NeedsApplication);
        }
        let dev = self.device.open().await?;
        let config = dev.active_configuration()?;
        let mut app_int = None;
        let mut reset_int = None;
        for interface in config.interfaces()? {
            let alternate = interface.alternate();
            match (alternate.class(), alternate.subclass()) {
                (CUSTOM_CLASS_ID, CUSTOM_SUBCLASS_ID) => {
                    app_int = Some(alternate.claim().await?);
                }
                (CUSTOM_CLASS_ID, PICOBOOT_SUBCLASS_ID) => {
                    reset_int = Some(alternate.claim().await?);
                }
                _ => {}
            }
        }
        let app_int = app_int.ok_or(UsbError::MissingInterface)?;
        let reset_int = reset_int.ok_or(UsbError::MissingInterface)?;

        let mut endpoints = app_int.endpoints().await?.into_iter();
        let response = endpoints
            .next()
            .ok_or(UsbError::MissingEndpoint)?
            .endpoint_in()?;
        let request = endpoints
            .next()
            .ok_or(UsbError::MissingEndpoint)?
            .endpoint_out()?;
        let status = endpoints
            .next()
            .ok_or(UsbError::MissingEndpoint)?
            .endpoint_in()?;

        Ok(UsbClient {
            request_response: Mutex::new(RequestResponse { request, response }),
            status: Mutex::new(status),
            reset_int: Mutex::new(reset_int),
        })
    }
    pub async fn connect_picoboot(self) -> Result<Picoboot, Error> {
        if self.device.product_id() != PICOBOOT_PRODUCT_ID {
            return Err(Error::NeedsPicoboot);
        }
        Ok(Picoboot::new(self.device).await?)
    }
}

impl UsbClient {
    pub async fn invoke_raw(&self, req: &[u8]) -> Result<Vec<u8>, Error> {
        let mut request_response = self.request_response.lock().await;
        request_response.request.write_all(req).await?;
        request_response.request.flush().await?;
        loop {
            let mut response = vec![];
            request_response
                .response
                .until_short_packet()
                .read_to_end(&mut response)
                .await?;
            info!("raw response ={:?}", response);
            if response.len() != 0 {
                return Ok(response);
            }
        }
    }

    pub async fn receive_status_raw(&self) -> Result<Vec<u8>, Error> {
        let mut status = self.status.lock().await;
        let mut response = vec![];
        assert!(!status.fill_buf().await?.is_empty());
        status
            .until_short_packet()
            .read_to_end(&mut response)
            .await?;
        info!("raw status ={:?}", response);
        Ok(response)
    }

    pub async fn reset_picoboot(&self) -> Result<(), Error> {
        let reset_int = self.reset_int.lock().await;
        reset_int
            .control_out(
                ControlOut {
                    control_type: ControlType::Class,
                    recipient: Recipient::Interface,
                    request: PICOBOOT_RESET_REQUEST_BOOTSEL,
                    value: 0,
                    index: reset_int.interface_number() as u16,
                    data: &[],
                },
                Duration::from_secs(10000),
            )
            .await?;
        Ok(())
    }
}

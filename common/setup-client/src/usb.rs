use crate::error::Error;
use log::info;
use nusb::io::{EndpointRead, EndpointWrite};
use nusb::transfer::Buffer;
use nusb::transfer::{Bulk, ControlOut, ControlType, In, Out, Recipient};
use nusb::{DeviceInfo, Endpoint, Interface, list_devices};
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
    request: EndpointWrite<Bulk>,
    response: Endpoint<Bulk, In>,
}

pub struct UsbClient {
    request_response: Mutex<RequestResponse>,
    status: Mutex<Endpoint<Bulk, In>>,
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
    pub fn from_device_info(device: DeviceInfo) -> UsbClientBuilder {
        UsbClientBuilder { device }
    }
    pub async fn list() -> Result<Vec<UsbClientBuilder>, Error> {
        Ok(list_devices()
            .await?
            .filter(|device| {
                device.vendor_id() == VENDOR_ID
                    && (device.product_id() == APPLICATION_PRODUCT_ID
                        || device.product_id() == PICOBOOT_PRODUCT_ID)
            })
            .map(Self::from_device_info)
            .collect())
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

        let app_int = config
            .interfaces()
            .find(|int| {
                int.first_alt_setting().class() == CUSTOM_CLASS_ID
                    && int.first_alt_setting().subclass() == CUSTOM_SUBCLASS_ID
            })
            .ok_or(UsbError::MissingInterface)?;
        let app_int = dev.claim_interface(app_int.interface_number()).await?;
        let desc = app_int.descriptor().ok_or(UsbError::MissingDescriptor)?;
        let mut ep = desc.endpoints();
        let response =
            app_int.endpoint::<Bulk, In>(ep.next().ok_or(UsbError::MissingEndpoint)?.address())?;
        let request = app_int
            .endpoint::<Bulk, Out>(ep.next().ok_or(UsbError::MissingEndpoint)?.address())?
            .writer(BUFFER_SIZE);
        let status =
            app_int.endpoint::<Bulk, In>(ep.next().ok_or(UsbError::MissingEndpoint)?.address())?;

        let reset_int = config
            .interfaces()
            .find(|int| {
                int.first_alt_setting().class() == CUSTOM_CLASS_ID
                    && int.first_alt_setting().subclass() == PICOBOOT_SUBCLASS_ID
            })
            .ok_or(UsbError::MissingInterface)?;
        let reset_int = dev.claim_interface(reset_int.interface_number()).await?;

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
        let response = self.receive_raw(&mut request_response.response).await?;
        Ok(response)
    }

    pub async fn receive_status_raw(&self) -> Result<Vec<u8>, Error> {
        let mut status = self.status.lock().await;
        let status = self.receive_raw(&mut *status).await?;
        Ok(status)
    }

    async fn receive_raw(&self, endpoint: &mut Endpoint<Bulk, In>) -> Result<Vec<u8>, Error> {
        let mut result = vec![];
        loop {
            let packet_size = endpoint.max_packet_size();
            endpoint.submit(Buffer::new(packet_size));
            let buffer = endpoint.next_complete().await.into_result()?.into_vec();
            result.extend_from_slice(&buffer);
            if buffer.len() < packet_size {
                break;
            }
        }
        Ok(result)
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

use crate::ble_connection::BleConnection;
use crate::error::Error;
use crate::status::{Status, StatusPriority};
use crate::utils::try_window;
use js_sys::futures::spawn_local;
use js_sys::{Array, DataView, Uint8Array};
use log::{error, info};
use protocol::ble::SERIAL_MTU;
use protocol::setup::{
    AppSettings, AppStatus, DeviceInfo, SetupRequest, SetupResponse, MAX_SETUP_MESSAGE_SIZE,
};
use protocol::usb::{CUSTOM_CLASS_ID, CUSTOM_SUBCLASS_ID, VENDOR_ID};
use serde::Deserialize;
use std::future::IntoFuture;
use std::rc::Rc;
use web_sys::{
    Usb, UsbDevice, UsbDeviceFilter, UsbDeviceRequestOptions, UsbEndpoint, UsbInterface,
    UsbTransferStatus,
};

pub struct UsbConnection {
    device: UsbDevice,
    interface: UsbInterface,
    request: UsbEndpoint,
    response: UsbEndpoint,
    status: UsbEndpoint,
}

impl UsbConnection {
    pub async fn new(connect_status: Rc<Status>) -> Result<Rc<UsbConnection>, Error> {
        let usb: Usb = try_window()?.navigator().usb();
        let mut filter = UsbDeviceFilter::new();
        filter.set_vendor_id(VENDOR_ID);
        let device = usb
            .request_device(&UsbDeviceRequestOptions::new(&[filter]))
            .await?;
        connect_status.set(
            StatusPriority::Info,
            "USB: Opening connection...".to_string(),
        );
        let config = device
            .configuration()
            .ok_or(Error::UsbConfigurationNotFound)?;
        let interfaces: Array<UsbInterface> = config.interfaces();
        device.open().await?;
        connect_status.set(
            StatusPriority::Info,
            "USB: Claiming interface...".to_string(),
        );
        for interface in interfaces {
            let alternate = interface.alternate();
            if alternate.interface_class() == CUSTOM_CLASS_ID
                && alternate.interface_subclass() == CUSTOM_SUBCLASS_ID
            {
                device.claim_interface(interface.interface_number()).await?;
                let mut endpoints = alternate.endpoints().into_iter();
                let response = endpoints.next().ok_or(Error::UsbMissingEndpoint)?;
                let request = endpoints.next().ok_or(Error::UsbMissingEndpoint)?;
                let status = endpoints.next().ok_or(Error::UsbMissingEndpoint)?;
                connect_status.set(StatusPriority::Info, "USB: Connected".to_string());
                return Ok(Rc::new(UsbConnection {
                    device,
                    interface,
                    request,
                    response,
                    status,
                }));
            }
        }
        Err(Error::UsbMissingInterface)
    }
    async fn receive_message<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &UsbEndpoint,
    ) -> Result<T, Error> {
        let mut response_buffer: Vec<u8> = vec![];
        let packet_size = endpoint.packet_size();
        loop {
            let result = self
                .device
                .transfer_in(endpoint.endpoint_number(), packet_size)
                .await?;
            match result.status() {
                UsbTransferStatus::Ok => {}
                e => return Err(Error::UsbTransferError(e)),
            }
            let result: DataView = result.data().ok_or(Error::UsbMissingData)?;
            let result = Uint8Array::new(&result.buffer()).to_vec();
            response_buffer.extend_from_slice(&result);
            if result.len() < packet_size as usize {
                break;
            }
        }
        let mut temp = vec![0u8; MAX_SETUP_MESSAGE_SIZE];
        let response = serde_json_core::from_slice_escaped::<T>(&response_buffer, &mut temp)?.0;
        Ok(response)
    }
    pub async fn invoke(&self, request: SetupRequest) -> Result<SetupResponse, Error> {
        let mut request_buffer = serde_json_core::to_vec::<_, MAX_SETUP_MESSAGE_SIZE>(&request)?;
        self.device
            .transfer_out_with_u8_slice(self.request.endpoint_number(), &mut request_buffer)?
            .await?;
        self.receive_message(&self.response).await
    }
    pub async fn next_status(&self) -> Result<AppStatus, Error> {
        Ok(self.receive_message(&self.status).await?)
    }
}

impl Drop for UsbConnection {
    fn drop(&mut self) {
        let promise = self.device.close();
        spawn_local(async move {
            if let Err(e) = promise.await {
                error!("Error closing usb connection: {:?}", e);
            }
        });
    }
}

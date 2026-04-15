pub mod error;
pub mod os;

use crate::Error;
use js_sys::Uint8Array;
use web_sys::UsbTransferStatus;

pub struct DeviceInfo {
    device: web_sys::UsbDevice,
}

pub struct Device {
    device: web_sys::UsbDevice,
}

pub struct Configuration {
    device: web_sys::UsbDevice,
    configuration: web_sys::UsbConfiguration,
}

#[derive(Clone)]
pub struct InterfaceInfo {
    device: web_sys::UsbDevice,
    interface: web_sys::UsbInterface,
}

pub struct Interface {
    device: web_sys::UsbDevice,
    interface: web_sys::UsbInterface,
}

pub struct EndpointInfo {
    device: web_sys::UsbDevice,
    endpoint: web_sys::UsbEndpoint,
}

pub struct EndpointIn {
    device: web_sys::UsbDevice,
    endpoint: web_sys::UsbEndpoint,
}

pub struct EndpointOut {
    device: web_sys::UsbDevice,
    endpoint: web_sys::UsbEndpoint,
}

impl DeviceInfo {
    pub async fn open(&self) -> Result<Device, Error> {
        self.device.open().into_future().await?;
        Ok(Device {
            device: self.device.clone(),
        })
    }
}

impl Device {
    pub fn active_configuration(&self) -> Result<Configuration, Error> {
        Ok(Configuration {
            device: self.device.clone(),
            configuration: self
                .device
                .configuration()
                .ok_or(Error::MissingDescriptor)?,
        })
    }
}

impl Configuration {
    pub async fn interfaces(&self) -> Result<Vec<InterfaceInfo>, Error> {
        Ok(self
            .configuration
            .interfaces()
            .into_iter()
            .map(|interface| InterfaceInfo {
                device: self.device.clone(),
                interface,
            })
            .collect())
    }
}

impl InterfaceInfo {
    pub async fn claim(&self) -> Result<Interface, Error> {
        self.device
            .claim_interface(self.interface.interface_number())
            .into_future()
            .await?;
        Ok(Interface {
            device: self.device.clone(),
            interface: self.interface.clone(),
        })
    }
}

impl Interface {
    pub async fn endpoints(&self) -> Result<Vec<EndpointInfo>, Error> {
        Ok(self
            .interface
            .alternate()
            .endpoints()
            .into_iter()
            .map(|endpoint| EndpointInfo {
                device: self.device.clone(),
                endpoint,
            })
            .collect())
    }
}

impl EndpointInfo {
    pub async fn endpoint_in(&self) -> Result<EndpointIn, Error> {
        Ok(EndpointIn {
            device: self.device.clone(),
            endpoint: self.endpoint.clone(),
        })
    }
    pub async fn endpoint_out(&self) -> Result<EndpointOut, Error> {
        Ok(EndpointOut {
            device: self.device.clone(),
            endpoint: self.endpoint.clone(),
        })
    }
}

impl EndpointIn {
    pub fn max_packet_size(&self) -> usize {
        self.endpoint.packet_size() as usize
    }
    pub async fn read_once(&mut self) -> Result<Vec<u8>, Error> {
        let result = self
            .device
            .transfer_in(self.endpoint.endpoint_number(), self.endpoint.packet_size())
            .await?;
        match result.status() {
            UsbTransferStatus::Ok => {}
            error => return Err(Error::TransferError(error::TransferError::from(error))),
        }
        let result = result.data().ok_or(Error::MissingData)?;
        let result = Uint8Array::new(&result.buffer()).to_vec();
        Ok(result)
    }
}

impl EndpointOut {
    pub fn max_packet_size(&self) -> usize {
        self.endpoint.packet_size() as usize
    }
    pub async fn write_once(&mut self, data: &[u8]) -> Result<(), Error> {
        let result = self
            .device
            .transfer_out_with_u8_slice(self.endpoint.endpoint_number(), &mut data.to_vec())?
            .await?;
        match result.status() {
            UsbTransferStatus::Ok => {}
            error => return Err(Error::TransferError(error::TransferError::from(error))),
        }
        if (result.bytes_written() as usize) < data.len() {
            return Err(Error::TruncatedWrite);
        }
        Ok(())
    }
}

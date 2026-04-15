pub mod os;

use crate::error::Error;
use crate::{EndpointDirection, TransferType};
use nusb::transfer::Buffer;

pub struct DeviceInfo {
    device_info: nusb::DeviceInfo,
}

pub struct Device {
    device: nusb::Device,
}

pub struct Configuration {
    interfaces: Vec<InterfaceInfo>,
}

#[derive(Clone)]
pub struct InterfaceInfo {
    interface_number: u8,
    alternate: AlternateInterfaceInfo,
    alternates: Vec<AlternateInterfaceInfo>,
}

#[derive(Clone)]
pub struct AlternateInterfaceInfo {
    device: nusb::Device,
    class: u8,
    subclass: u8,
    interface_number: u8,
}

pub struct Interface {
    device: nusb::Device,
    interface: nusb::Interface,
}

pub struct EndpointInfo {
    device: nusb::Device,
    interface: nusb::Interface,
    address: u8,
}

pub struct EndpointIn {
    endpoint: nusb::Endpoint<nusb::transfer::Bulk, nusb::transfer::In>,
}

pub struct EndpointOut {
    endpoint: nusb::Endpoint<nusb::transfer::Bulk, nusb::transfer::Out>,
}

impl DeviceInfo {
    pub async fn open(&self) -> Result<Device, Error> {
        Ok(Device {
            device: self.device_info.open().await?,
        })
    }
}

impl Device {
    pub fn active_configuration(&self) -> Result<Configuration, Error> {
        Ok(Configuration::from_nusb(
            &self.device,
            self.device.active_configuration()?,
        ))
    }
    pub fn configurations(&self) -> Result<Vec<Configuration>, Error> {
        Ok(self
            .device
            .configurations()
            .map(|c| Configuration::from_nusb(&self.device, c))
            .collect())
    }
}

impl Configuration {
    fn from_nusb(
        device: &nusb::Device,
        configuration: nusb::descriptors::ConfigurationDescriptor<'_>,
    ) -> Self {
        let interfaces = configuration
            .interfaces()
            .map(|interface| {
                InterfaceInfo {
                    interface_number: interface.interface_number(),
                    alternate: AlternateInterfaceInfo::from_nusb(
                        &device,
                        &interface,
                        interface.first_alt_setting(),
                    ),
                    alternates: interface
                        .alt_settings()
                        .map(|setting| {
                            AlternateInterfaceInfo::from_nusb(&device, &interface, setting)
                        })
                        .collect(),
                }
                //
            })
            .collect();
        Configuration { interfaces }
    }
    pub fn interfaces(&self) -> Result<Vec<InterfaceInfo>, Error> {
        Ok(self.interfaces.clone())
    }
}

impl InterfaceInfo {
    pub fn alternate(&self) -> AlternateInterfaceInfo {
        self.alternate.clone()
    }
    pub fn alternates(&self) -> Vec<AlternateInterfaceInfo> {
        self.alternates.clone()
    }
}

impl AlternateInterfaceInfo {
    fn from_nusb(
        device: &nusb::Device,
        interface: &nusb::descriptors::InterfaceDescriptors<'_>,
        alternate: nusb::descriptors::InterfaceDescriptor<'_>,
    ) -> Self {
        AlternateInterfaceInfo {
            device: device.clone(),
            class: alternate.class(),
            subclass: alternate.subclass(),
            interface_number: interface.interface_number(),
        }
    }
    pub fn interface_number(&self) -> u8 {
        self.interface_number
    }
    pub fn class(&self) -> u8 {
        self.class
    }
    pub fn subclass(&self) -> u8 {
        self.subclass
    }
    pub async fn claim(&self) -> Result<Interface, Error> {
        Ok(Interface {
            device: self.device.clone(),
            interface: self.device.claim_interface(self.interface_number).await?,
        })
    }
}

impl Interface {
    pub fn interface_number(&self) -> u8 {
        self.interface.interface_number()
    }
    pub fn endpoints(&self) -> Result<Vec<EndpointInfo>, Error> {
        Ok(self
            .interface
            .descriptor()
            .ok_or(Error::MissingDescriptor)?
            .endpoints()
            .map(|endpoint| EndpointInfo {
                device: self.device.clone(),
                interface: self.interface.clone(),
                address: endpoint.address(),
            })
            .collect())
    }
}

impl EndpointInfo {
    pub fn transfer_type(&self) -> TransferType {
        todo!();
    }
    pub fn direction(&self) -> EndpointDirection {
        todo!();
    }
    pub fn endpoint_in(&self) -> Result<EndpointIn, Error> {
        Ok(EndpointIn {
            endpoint: self.interface.endpoint(self.address)?,
        })
    }
    pub fn endpoint_out(&self) -> Result<EndpointOut, Error> {
        Ok(EndpointOut {
            endpoint: self.interface.endpoint(self.address)?,
        })
    }
}

impl EndpointIn {
    pub fn max_packet_size(&self) -> usize {
        self.endpoint.max_packet_size()
    }
    pub async fn read_once(&mut self) -> Result<Vec<u8>, Error> {
        let buffer = Buffer::new(self.endpoint.max_packet_size());
        self.endpoint.submit(buffer);
        let buffer = self
            .endpoint
            .next_complete()
            .await
            .into_result()?
            .into_vec();
        Ok(buffer)
    }
}

impl EndpointOut {
    pub fn max_packet_size(&self) -> usize {
        self.endpoint.max_packet_size()
    }
    pub async fn write_once(&mut self, data: &[u8]) -> Result<(), Error> {
        assert_eq!(data.len(), self.endpoint.max_packet_size());
        let buffer = Buffer::from(data);
        self.endpoint.submit(buffer);
        let buffer = self.endpoint.next_complete().await;
        buffer.into_result()?;
        Ok(())
    }
}

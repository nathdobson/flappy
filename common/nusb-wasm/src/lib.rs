use crate::error::Error;
use std::time::Duration;
use thiserror::Error;

#[cfg(not(target_family = "wasm"))]
mod native;

pub mod error;
#[cfg(target_family = "wasm")]
mod wasm;

mod platform {

    #[cfg(target_family = "wasm")]
    pub use crate::wasm::*;

    #[cfg(not(target_family = "wasm"))]
    pub use crate::native::*;
}

pub mod os {
    #[cfg(target_family = "wasm")]
    pub use crate::wasm::os::*;

    #[cfg(not(target_family = "wasm"))]
    pub use crate::native::os::*;
}

pub struct DeviceInfo(platform::DeviceInfo);

pub struct Device(platform::Device);

pub struct InterfaceInfo(platform::InterfaceInfo);

pub struct AlternateInterfaceInfo(platform::AlternateInterfaceInfo);

pub struct Configuration(platform::Configuration);

pub struct Interface(platform::Interface);

pub struct EndpointInfo(platform::EndpointInfo);

pub struct EndpointIn(platform::EndpointIn);

pub struct EndpointOut(platform::EndpointOut);

pub enum ControlType {
    Class,
    Vendor,
}

pub enum Recipient {
    Interface,
}

pub enum TransferType {
    Bulk,
}

pub enum EndpointDirection {
    In,
    Out,
}

pub struct ControlOut<'a> {
    control_type: ControlType,
    recipient: Recipient,
    request: u8,
    value: u8,
    index: u16,
    data: &'a [u8],
}

pub struct ControlIn {
    control_type: ControlType,
    recipient: Recipient,
    request: u8,
    value: u8,
    index: u16,
    length: u16,
}

impl DeviceInfo {
    pub fn product_id(&self) -> u16 {
        todo!();
    }
    pub async fn open(&self) -> Result<Device, Error> {
        Ok(Device(self.0.open().await?))
    }
}

impl Device {
    pub fn active_configuration(&self) -> Result<Configuration, Error> {
        Ok(Configuration(self.0.active_configuration()?))
    }
    pub fn configurations(&self) -> Result<Vec<Configuration>, Error> {
        Ok(self
            .0
            .configurations()?
            .into_iter()
            .map(Configuration)
            .collect())
    }
}

impl Configuration {
    pub fn interfaces(&self) -> Result<Vec<InterfaceInfo>, Error> {
        Ok(self
            .0
            .interfaces()?
            .into_iter()
            .map(InterfaceInfo)
            .collect())
    }
}

impl InterfaceInfo {
    pub fn alternate(&self) -> AlternateInterfaceInfo {
        AlternateInterfaceInfo(self.0.alternate())
    }
    pub fn alternates(&self) -> Vec<AlternateInterfaceInfo> {
        self.0
            .alternates()
            .into_iter()
            .map(AlternateInterfaceInfo)
            .collect()
    }
}

impl AlternateInterfaceInfo {
    pub fn interface_number(&self) -> u8 {
        self.0.interface_number()
    }
    pub fn class(&self) -> u8 {
        self.0.class()
    }
    pub fn subclass(&self) -> u8 {
        self.0.subclass()
    }
    pub async fn claim(&self) -> Result<Interface, Error> {
        Ok(Interface(self.0.claim().await?))
    }
}

impl Interface {
    pub fn interface_number(&self) -> u8 {
        self.0.interface_number()
    }
    pub async fn control_out(&self, control_out: ControlOut<'_>) -> Result<(), Error> {
        todo!();
    }
    pub async fn control_in(&self, p0: ControlIn) -> Result<Vec<u8>, Error> {
        todo!()
    }
    pub fn endpoints(&self) -> Result<Vec<EndpointInfo>, Error> {
        Ok(self.0.endpoints()?.into_iter().map(EndpointInfo).collect())
    }
}

impl EndpointInfo {
    pub fn transfer_type(&self) -> TransferType {
        self.0.transfer_type()
    }
    pub fn direction(&self) -> EndpointDirection {
        self.0.direction()
    }
    pub fn endpoint_in(&self) -> Result<EndpointIn, Error> {
        Ok(EndpointIn(self.0.endpoint_in()?))
    }
    pub fn endpoint_out(&self) -> Result<EndpointOut, Error> {
        Ok(EndpointOut(self.0.endpoint_out()?))
    }
}

impl EndpointIn {
    pub fn max_packet_size(&self) -> usize {
        self.0.max_packet_size()
    }
    pub async fn read_once(&mut self) -> Result<Vec<u8>, Error> {
        self.0.read_once().await
    }
    pub async fn read_until_short(&mut self) -> Result<Vec<u8>, Error> {
        let mut result = vec![];
        loop {
            let next = self.read_once().await?;
            result.extend_from_slice(&next);
            if next.len() < self.max_packet_size() {
                break;
            }
        }
        Ok(result)
    }
    pub async fn clear_halt(&mut self) -> Result<(), Error> {
        todo!();
    }
}

impl EndpointOut {
    pub fn max_packet_size(&self) -> usize {
        self.0.max_packet_size()
    }
    pub async fn write_once(&mut self, data: &[u8]) -> Result<(), Error> {
        self.0.write_once(data).await
    }
    pub async fn write_all(&mut self, data: &[u8]) -> Result<(), Error> {
        for chunk in data.chunks(self.max_packet_size()) {
            self.write_once(chunk).await?;
        }
        if data.len() % self.max_packet_size() == 0 {
            // Ensure the write_all operation always ends with a write shorter than max_packet_size,
            // so the other side knows the boundary.
            self.write_once(&[]).await?;
        }
        Ok(())
    }
    pub async fn clear_halt(&mut self) -> Result<(), Error> {
        todo!();
    }
}

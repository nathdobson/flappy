use crate::error::Error;
// use btleplug::api::{
//     Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter, WriteType,
//     bleuuid::uuid_from_u16,
// };
// use btleplug::platform::{Adapter, Manager, Peripheral};
// use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
// use btleplug::platform::{Adapter, Manager, Peripheral, PeripheralId};
// use bluest::{Adapter, Service};
use btleplug::api::{
    Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter, Service,
    ValueNotification, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral, PeripheralId};
use futures_core::Stream;
use futures_util::FutureExt;
use futures_util::StreamExt;
use protocol::ble::SERIAL_MTU;
use protocol::setup::WriteSettingsError::SerdeError;
use protocol::setup::{AppStatus, MAX_SETUP_MESSAGE_SIZE, SetupRequest, SetupResponse};
use serde_json_core::heapless;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::pin::Pin;
use std::thread;
use std::time::Duration;
use tokio::time;
use uuid::{Uuid, uuid};

pub const FLAPPY_SERVICE_UUID: Uuid = uuid!("5af0b930-b9b5-11f0-b558-0800200c9a66");
pub const SERIAL_OUT_UUID: Uuid = uuid!("2d2bc907-c9fa-49fd-ba45-410cddf61e5c");
pub const SERIAL_IN_UUID: Uuid = uuid!("4574529b-fbe4-44ae-ba52-d877ac76ef2d");
pub const APP_STATUS_UUID: Uuid = uuid!("4dc5669d-6bc8-40eb-b6af-8091d4e9b713");

pub struct BleAddress {
    pub peripheral: Peripheral,
}

pub struct BleConnection {
    service: Service,
    peri: Peripheral,
    serial_out: Characteristic,
    serial_in: Characteristic,
    notifications: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
}

async fn get_central() -> Result<Adapter, Error> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let central = adapters
        .into_iter()
        .nth(0)
        .ok_or(Error::BleAdapterNotFound)?;

    Ok(central)
}

impl BleAddress {
    pub fn try_to_string(&self) -> Result<String, Error> {
        Ok(serde_string::to_string(&self.peripheral.id())?)
    }
    pub async fn list()
    -> Result<Pin<Box<dyn Stream<Item = Result<BleAddress, Error>> + Send>>, Error> {
        let central = get_central().await?;
        central.start_scan(ScanFilter::default()).await?;
        Ok(Box::pin(central.events().await?.filter_map(move |x| {
            let central = central.clone();
            async move {
                match x {
                    CentralEvent::DeviceDiscovered(id) => Some(
                        try {
                            let peripheral = central.peripheral(&id).await?;
                            let properties = peripheral.properties().await?;
                            if let Some(properties) = properties {
                                if properties.services.contains(&FLAPPY_SERVICE_UUID) {
                                    BleAddress { peripheral }
                                } else {
                                    return None;
                                }
                            } else {
                                return None;
                            }
                        },
                    ),
                    _ => None,
                }
            }
        })))
    }
}

impl BleConnection {
    pub async fn new(address: &str) -> Result<Self, Error> {
        let central = get_central().await?;
        central.start_scan(ScanFilter::default()).await?;
        let peri = loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            match central.peripheral(&serde_string::from_str(address)?).await {
                Ok(peri) => break peri,
                Err(btleplug::Error::DeviceNotFound) => continue,
                Err(e) => return Err(e.into()),
            }
        };
        peri.connect().await?;
        peri.discover_services().await?;
        for service in peri.services() {
            let service: Service = service;
            if service.uuid == FLAPPY_SERVICE_UUID {
                let mut serial_out = None;
                let mut serial_in = None;
                let mut app_status = None;
                for c in &service.characteristics {
                    if c.uuid == SERIAL_OUT_UUID {
                        serial_out = Some(c.clone());
                    } else if c.uuid == SERIAL_IN_UUID {
                        serial_in = Some(c.clone());
                    } else if c.uuid == APP_STATUS_UUID {
                        app_status = Some(c.clone());
                    }
                }
                let serial_out = serial_out.ok_or(Error::MissingCharacteristic)?;
                let serial_in = serial_in.ok_or(Error::MissingCharacteristic)?;
                let app_status = app_status.ok_or(Error::MissingCharacteristic)?;
                peri.subscribe(&serial_in).await?;
                peri.subscribe(&app_status).await?;
                let notifications = peri.notifications().await?;
                return Ok(BleConnection {
                    peri,
                    service,
                    serial_out,
                    serial_in,
                    notifications,
                });
            }
        }
        Err(Error::MissingService)
    }
    pub async fn invoke(&mut self, request: &SetupRequest) -> Result<SetupResponse, Error> {
        let mut receive_buffer = heapless::Vec::<u8, MAX_SETUP_MESSAGE_SIZE>::new();
        let mut tmp = [0u8; MAX_SETUP_MESSAGE_SIZE];
        let request = serde_json_core::to_vec::<_, MAX_SETUP_MESSAGE_SIZE>(request)?;
        for chunk in request.chunks(SERIAL_MTU) {
            self.peri
                .write(&self.serial_out, chunk, WriteType::WithoutResponse)
                .await?;
        }
        if request.len() % SERIAL_MTU == 0 {
            self.peri
                .write(&self.serial_out, &[], WriteType::WithoutResponse)
                .await?;
        }
        loop {
            let next = self
                .notifications
                .next()
                .await
                .ok_or(Error::MissingNotification)?;
            if next.uuid == SERIAL_IN_UUID {
                receive_buffer.extend_from_slice(&next.value)?;
                if next.value.len() < SERIAL_MTU {
                    let response = serde_json_core::from_slice_escaped::<SetupResponse>(
                        &receive_buffer,
                        &mut tmp,
                    )?
                    .0;
                    return Ok(response);
                }
            }
        }
    }
    pub async fn receive(&mut self) -> Result<AppStatus, Error> {
        let mut tmp = [0u8; MAX_SETUP_MESSAGE_SIZE];
        loop {
            let next = self
                .notifications
                .next()
                .await
                .ok_or(Error::MissingNotification)?;
            if next.uuid == APP_STATUS_UUID {
                return Ok(serde_json_core::from_slice_escaped::<AppStatus>(
                    &next.value,
                    &mut tmp,
                )?
                .0);
            }
        }
    }
}

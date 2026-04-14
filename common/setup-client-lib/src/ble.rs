use crate::error::Error;
use crate::serde;
use btleplug::api::ScanFilter;
use btleplug::api::{Central, CentralEvent};
use btleplug::api::{Central as _, Peripheral as _};
use btleplug::api::{Characteristic, Manager as _, Service, ValueNotification, WriteType};
use btleplug::platform::{Adapter, PeripheralId};
use btleplug::platform::{Manager, Peripheral};
use futures_util::Stream;
use futures_util::stream::StreamExt;
use protocol::ble::{
    APP_STATUS_UUID, FLAPPY_SERVICE_UUID, SERIAL_IN_UUID, SERIAL_MTU, SERIAL_OUT_UUID,
};
use protocol::setup::{AppStatus, MAX_SETUP_MESSAGE_SIZE, SetupRequest, SetupResponse};
use std::mem;
use std::pin::Pin;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{Mutex, mpsc, watch};

#[derive(Debug)]
pub struct BleClientBuilder {
    peripheral: Peripheral,
    address: String,
}

#[derive(Debug, Error)]
pub enum BleError {
    #[error("service is missing required characteristic")]
    MissingCharacteristic,
    #[error("device is missing required service")]
    MissingService,
    #[error("stream ended prematurely")]
    UnexpectedEndOfStream,
}

pub struct BleClient {
    peripheral: Peripheral,
    serial_out: Characteristic,
    status_rx: Mutex<mpsc::UnboundedReceiver<Vec<u8>>>,
    serial_in_rx: Mutex<mpsc::UnboundedReceiver<Vec<u8>>>,
}

impl BleClientBuilder {
    pub async fn scan()
    -> Result<Pin<Box<dyn Stream<Item = Result<BleClientBuilder, Error>>>>, Error> {
        let manager = Manager::new().await?;
        let adapters = manager.adapters().await?;
        let central = adapters
            .into_iter()
            .nth(0)
            .ok_or(btleplug::Error::NoAdapterAvailable)?;
        let events = central.events().await?;
        central
            .start_scan(ScanFilter {
                services: vec![FLAPPY_SERVICE_UUID],
            })
            .await?;
        Ok(Box::pin(events.filter_map(move |event| {
            let central = central.clone();
            async move { Self::handle_event(central, event).await.transpose() }
        })))
    }
    pub async fn handle_event(
        central: Adapter,
        event: CentralEvent,
    ) -> Result<Option<BleClientBuilder>, Error> {
        if let CentralEvent::DeviceDiscovered(id) = event {
            let peripheral = central.peripheral(&id).await?;
            let address = serde_string::to_string(&peripheral.id())?;
            return Ok(Some(BleClientBuilder {
                peripheral,
                address,
            }));
        }

        Ok(None)
    }
    pub fn address(&self) -> &str {
        &self.address
    }
    pub async fn connect(self) -> Result<BleClient, Error> {
        self.peripheral.connect().await?;
        self.peripheral.discover_services().await?;
        let service = self
            .peripheral
            .services()
            .into_iter()
            .find(|service| service.uuid == FLAPPY_SERVICE_UUID)
            .ok_or(BleError::MissingService)?;

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
        let serial_out = serial_out.ok_or(BleError::MissingCharacteristic)?;
        let serial_in = serial_in.ok_or(BleError::MissingCharacteristic)?;
        let app_status = app_status.ok_or(BleError::MissingCharacteristic)?;
        self.peripheral.subscribe(&serial_in).await?;
        self.peripheral.subscribe(&app_status).await?;
        let notifications = self.peripheral.notifications().await?;
        let (status_tx, status_rx) = mpsc::unbounded_channel();
        let (serial_in_tx, serial_in_rx) = mpsc::unbounded_channel();
        spawn_local::spawn(BleClient::receive_notifications(
            notifications,
            status_tx,
            serial_in_tx,
        ));
        return Ok(BleClient {
            peripheral: self.peripheral,
            serial_out,
            status_rx: Mutex::new(status_rx),
            serial_in_rx: Mutex::new(serial_in_rx),
        });
    }
}

impl BleClient {
    async fn receive_notifications(
        mut notifications: impl Unpin + Stream<Item = ValueNotification>,
        status: mpsc::UnboundedSender<Vec<u8>>,
        serial_in: mpsc::UnboundedSender<Vec<u8>>,
    ) {
        let mut receive_buffer = Vec::new();
        while let Some(next) = notifications.next().await {
            match next.uuid {
                SERIAL_IN_UUID => {
                    receive_buffer.extend_from_slice(&next.value);
                    if next.value.len() < SERIAL_MTU {
                        serial_in
                            .send(mem::replace(&mut receive_buffer, Vec::new()))
                            .ok();
                    }
                }
                APP_STATUS_UUID => {
                    status.send(next.value).ok();
                }
                _ => {}
            }
        }
    }
    pub async fn invoke_raw(&self, request: &[u8]) -> Result<Vec<u8>, Error> {
        let mut receiver = self.serial_in_rx.lock().await;
        for chunk in request.chunks(SERIAL_MTU) {
            self.peripheral
                .write(&self.serial_out, chunk, WriteType::WithoutResponse)
                .await?;
        }
        if request.len() % SERIAL_MTU == 0 {
            self.peripheral
                .write(&self.serial_out, &[], WriteType::WithoutResponse)
                .await?;
        }
        receiver
            .recv()
            .await
            .ok_or(BleError::UnexpectedEndOfStream.into())
    }
    pub async fn receive_status_raw(&self) -> Result<Vec<u8>, Error> {
        let mut status_in_rx = self.status_rx.lock().await;
        let result = status_in_rx
            .recv()
            .await
            .ok_or(BleError::UnexpectedEndOfStream)?;
        Ok(result)
    }
}

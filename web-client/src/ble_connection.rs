use crate::error::Error;
use crate::event_listener::{EventListener, EventType};
use crate::status::{Status, StatusPriority};
use crate::utils::{bluetooth, sleep};
use btleplug::api::Central;
use btleplug::api::CentralEvent;
use btleplug::api::Manager;
use btleplug::api::Peripheral;
use btleplug::api::ScanFilter;
use futures_util::StreamExt;
use js_sys::{ArrayBuffer, Uint8Array};
use log::{error, info};
use protocol::ble::{
    APP_STATUS_UUID, FLAPPY_SERVICE_UUID, SERIAL_IN_UUID, SERIAL_MTU, SERIAL_OUT_UUID,
};
use protocol::setup::{
    AppSettings, AppStatus, DeviceInfo, MAX_SETUP_MESSAGE_SIZE, SetupRequest, SetupResponse,
};
use std::cell::{OnceCell, RefCell};
use std::future::IntoFuture;
use std::rc::Rc;
use tokio::sync::{Mutex, mpsc, watch};
use wasm_bindgen::sys::Undefined;
use web_sys::{
    BluetoothLeScanFilterInit, BluetoothRemoteGattCharacteristic, BluetoothRemoteGattServer, Event,
    HtmlDivElement, RequestDeviceOptions, Text,
};
pub struct BleConnection {
    connect_status: Rc<Status>,
    server: BluetoothRemoteGattServer,
    status_char: BluetoothRemoteGattCharacteristic,
    serial_in_char: BluetoothRemoteGattCharacteristic,
    serial_in_buffer: RefCell<Vec<u8>>,
    serial_out_char: BluetoothRemoteGattCharacteristic,
    status_notify_listener: OnceCell<EventListener<'static>>,
    serial_in_notify_listener: OnceCell<EventListener<'static>>,
    response_rx: Mutex<mpsc::UnboundedReceiver<SetupResponse>>,
    response_tx: mpsc::UnboundedSender<SetupResponse>,
    app_status_rx: Mutex<watch::Receiver<AppStatus>>,
    app_status_tx: watch::Sender<AppStatus>,
}

impl BleConnection {
    pub async fn new(connect_status: Rc<Status>) -> Result<Rc<BleConnection>, Error> {
        let manager = btleplug::platform::Manager::new().await?;
        let adapters = manager.adapters().await?;
        let central = adapters
            .into_iter()
            .nth(0)
            .ok_or(Error::BluetoothNotSupported)?;
        let mut stream = central.events().await?;
        central
            .start_scan(ScanFilter {
                services: vec![FLAPPY_SERVICE_UUID],
            })
            .await?;
        while let Some(next) = stream.next().await {
            let central = central.clone();
            match next {
                CentralEvent::DeviceDiscovered(id) => {
                    let peripheral = central.peripheral(&id).await.map_err(Error::from)?;
                    let properties = peripheral.properties().await.map_err(Error::from)?;
                    if let Some(properties) = properties {
                        if properties.services.contains(&FLAPPY_SERVICE_UUID) {
                            info!("{:?}", peripheral);
                        }
                    }
                }
                _ => {}
            }
        }
        todo!();

        let bluetooth = bluetooth()?;
        let options = RequestDeviceOptions::new();
        let filter = BluetoothLeScanFilterInit::new();
        let flappy_service = FLAPPY_SERVICE_UUID.to_string();
        filter.set_services(&[flappy_service.clone().into()]);
        options.set_filters(&[filter]);
        connect_status.set(
            StatusPriority::Info,
            "Bluetooth: looking for devices...".to_string(),
        );
        let device = bluetooth.request_device(&options).into_future().await?;
        connect_status.set(
            StatusPriority::Info,
            "Bluetooth: connecting to device...".to_string(),
        );
        let gatt = device.gatt().ok_or(Error::CannotFindElement)?;
        let gatt = gatt.connect().into_future().await?;
        connect_status.set(
            StatusPriority::Info,
            "Bluetooth: connecting to service...".to_string(),
        );
        let service = gatt
            .get_primary_service_with_str(&flappy_service)
            .into_future()
            .await?;
        connect_status.set(
            StatusPriority::Info,
            "Bluetooth: retrieving characteristics...".to_string(),
        );
        let status_char = service
            .get_characteristic_with_str(&APP_STATUS_UUID.to_string())
            .into_future()
            .await?;
        let serial_in_char = service
            .get_characteristic_with_str(&SERIAL_IN_UUID.to_string())
            .into_future()
            .await?;
        let serial_out_char = service
            .get_characteristic_with_str(&SERIAL_OUT_UUID.to_string())
            .into_future()
            .await?;
        connect_status.set(
            StatusPriority::Info,
            "Bluetooth: listening to status updates...".to_string(),
        );
        let (response_tx, response_rx) = mpsc::unbounded_channel();
        let (app_status_tx, app_status_rx) = watch::channel(AppStatus::default());
        let connection = Rc::new(BleConnection {
            connect_status,
            server: gatt,
            status_char: status_char.clone(),
            serial_in_char: serial_in_char.clone(),
            serial_in_buffer: RefCell::new(vec![]),
            serial_out_char: serial_out_char.clone(),
            serial_in_notify_listener: OnceCell::new(),
            status_notify_listener: OnceCell::new(),
            response_tx,
            app_status_rx: Mutex::new(app_status_rx),
            response_rx: Mutex::new(response_rx),
            app_status_tx,
        });
        connection.connect_status.set(
            StatusPriority::Info,
            "Bluetooth: listening to status characteristic...".to_string(),
        );
        connection
            .status_notify_listener
            .set(EventListener::new(
                &status_char,
                EventType::CharacteristicValueChanged,
                {
                    let connection = Rc::downgrade(&connection);
                    move |e| {
                        if let Some(connection) = connection.upgrade() {
                            if let Err(e) = connection.update_status() {
                                connection
                                    .connect_status
                                    .set(StatusPriority::Error, format!("Bluetooth: {}", e));
                            }
                        }
                        false
                    }
                },
            )?)
            .ok()
            .unwrap();
        connection.connect_status.set(
            StatusPriority::Info,
            "Bluetooth: listening to serial input characteristic...".to_string(),
        );
        connection
            .serial_in_notify_listener
            .set(EventListener::new(
                &serial_in_char,
                EventType::CharacteristicValueChanged,
                {
                    let connection = Rc::downgrade(&connection);
                    move |e| {
                        if let Some(connection) = connection.upgrade() {
                            if let Err(e) = connection.update_serial_in() {
                                connection
                                    .connect_status
                                    .set(StatusPriority::Error, format!("Bluetooth: {}", e));
                            }
                        }
                        false
                    }
                },
            )?)
            .ok()
            .unwrap();
        connection.connect_status.set(
            StatusPriority::Info,
            "Waiting for status notifications...".to_string(),
        );
        status_char.start_notifications().into_future().await?;
        connection.connect_status.set(
            StatusPriority::Info,
            "Waiting for serial input notifications...".to_string(),
        );
        serial_in_char.start_notifications().into_future().await?;

        connection.connect_status.set(
            StatusPriority::Info,
            "Press white button on microcontroller.".to_string(),
        );
        connection.invoke(SetupRequest::Ping).await?;
        connection
            .connect_status
            .set(StatusPriority::Info, "Bluetooth: Connected".to_string());

        Ok(connection)
    }
    fn update_status(&self) -> Result<(), Error> {
        info!("Receiving status");
        let value = self.status_char.value().ok_or(Error::MissingStatusValue)?;
        let value: ArrayBuffer = value.buffer();
        let value = Uint8Array::new(&value).to_vec();
        if value.len() > 0 {
            let mut temp = vec![0u8; MAX_SETUP_MESSAGE_SIZE];
            let result = serde_json_core::from_slice_escaped::<AppStatus>(&value, &mut temp)?.0;
            self.app_status_tx.send(result).ok();
        }
        Ok(())
    }
    pub async fn next_status(&self) -> Result<AppStatus, Error> {
        let mut app_status_rx = self.app_status_rx.lock().await;
        app_status_rx.changed().await.ok();
        let result = app_status_rx.borrow().clone();
        Ok(result)
    }
    fn update_serial_in(&self) -> Result<(), Error> {
        let value = self
            .serial_in_char
            .value()
            .ok_or(Error::MissingStatusValue)?;
        let value: ArrayBuffer = value.buffer();
        let value = Uint8Array::new(&value).to_vec();
        let mut serial_in_buffer = self.serial_in_buffer.borrow_mut();
        let len = value.len();
        serial_in_buffer.extend(value);
        if len < SERIAL_MTU {
            let mut temp = vec![0u8; MAX_SETUP_MESSAGE_SIZE];
            let response =
                serde_json_core::from_slice_escaped::<SetupResponse>(&serial_in_buffer, &mut temp)?
                    .0;
            serial_in_buffer.clear();
            self.response_tx.send(response)?;
        }

        Ok(())
    }
    pub async fn invoke(&self, request: SetupRequest) -> Result<SetupResponse, Error> {
        let mut response = self.response_rx.lock().await;
        let buffer = serde_json_core::to_vec::<_, MAX_SETUP_MESSAGE_SIZE>(&request)?;
        for chunk in buffer.chunks(SERIAL_MTU) {
            self.send(chunk).await?;
        }
        if buffer.len() % SERIAL_MTU == 0 {
            self.send(&[]).await?;
        }
        info!("Finished sending");
        Ok(response.recv().await.ok_or(Error::ChannelClosed)?)
    }

    async fn send(&self, chunk: &[u8]) -> Result<(), Error> {
        let x: Undefined = self
            .serial_out_char
            .write_value_with_u8_array(&Uint8Array::new_from_slice(chunk))?
            .into_future()
            .await?;
        Ok(())
    }
}

impl Drop for BleConnection {
    fn drop(&mut self) {
        self.server.disconnect();
    }
}

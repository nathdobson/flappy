use crate::ble::{BleHandler, BleModule};
use crate::display::Display;
use crate::driver::DriverModule;
use crate::error::Error;
use crate::flash::{FlashModule, FlashSettings};
use crate::led::LedModule;
use crate::mqtt::{MqttHandler, MqttModule, MqttStatus};
use crate::peripherals::AppPeripherals;
use crate::product::{built_info, serial_number};
use crate::radio::RadioModule;
use crate::wifi::{WifiHandler, WifiModule, WifiStatus};
use core::cell::RefCell;
use core::future::pending;
use embassy_executor::Spawner;
use embassy_rp::clocks::RoscRng;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use heapless::{String, format};
use log::{error, info};
use serde::{Deserialize, Serialize};
use static_cell::make_static;

pub const MODULE: &'static str = "[APP  ]";
pub struct Application {
    spawner: Spawner,
    flash: &'static FlashModule,
    ble: &'static BleModule,
    led: &'static LedModule,
    wifi: &'static WifiModule,
    mqtt: &'static MqttModule,
    driver: &'static DriverModule,
    state: RefCell<FlashSettings>,
    wifi_status: Signal<NoopRawMutex, WifiStatus>,
    mqtt_status: Signal<NoopRawMutex, MqttStatus>,
    display_message: Signal<NoopRawMutex, MqttRequest>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MqttRequest {
    msg: String<128>,
}

impl MqttHandler for Application {
    fn handle_status(&self, status: MqttStatus) {
        self.mqtt_status.signal(status);
    }

    fn handle(&self, topic: &str, message: &[u8]) {
        if let Ok(message) = str::from_utf8(message) {
            info!("{MODULE} Received message on topic {}:", topic);
            for line in message.split('\n') {
                info!("{MODULE}      {}", line);
            }
        }
        match serde_json_core::from_slice::<MqttRequest>(message) {
            Ok((message, _)) => {
                info!("{MODULE} Parsed message as {:?}", message);
                self.display_message.signal(message);
            }
            Err(e) => error!("{MODULE} Cannot parse message {:?}", e),
        }
    }
}

fn trim_null<const N: usize>(mut x: String<N>) -> String<N> {
    if x.as_bytes().last() == Some(&b'\0') {
        x.pop();
    }
    x
}

impl BleHandler for Application {
    fn ble_handle_gatt_write(&self, id: u16) {
        let service = &self.ble.server().flappy_service;
        let ref mut state = *self.state.borrow_mut();
        let mut updated = false;
        if id == service.wifi_password.handle {
            updated = true;
            state.wifi.ssid = trim_null(self.ble.get(&service.wifi_ssid).unwrap_or_default());
            state.wifi.password =
                trim_null(self.ble.get(&service.wifi_password).unwrap_or_default());
            self.wifi.set_settings(state.wifi.clone());
            info!("{MODULE} Updating WiFi settings");
        } else if id == service.mqtt_topic.handle {
            updated = true;
            state.mqtt.hostname =
                trim_null(self.ble.get(&service.mqtt_hostname).unwrap_or_default());
            let port = trim_null(self.ble.get(&service.mqtt_port).unwrap_or_default());
            let port = &port;
            let port = port.strip_prefix("\"").unwrap_or(port);
            let port = port.strip_suffix("\"").unwrap_or(port);
            let port: u16 = port.parse().unwrap_or_default();
            state.mqtt.port = port;
            state.mqtt.username =
                trim_null(self.ble.get(&service.mqtt_username).unwrap_or_default());
            state.mqtt.password =
                trim_null(self.ble.get(&service.mqtt_password).unwrap_or_default());
            state.mqtt.topic = trim_null(self.ble.get(&service.mqtt_topic).unwrap_or_default());
            self.mqtt.set_settings(state.mqtt.clone());
            info!("{MODULE} Updating MQTT settings");
        }
        if updated {
            if let Err(e) = self.flash.save(state) {
                error!("{MODULE} failed to update wifi settings in flash {}", e);
            }
        }
    }
}

impl WifiHandler for Application {
    fn handle_wifi_status(&self, status: WifiStatus) {
        self.wifi_status.signal(status);
    }
}

impl Application {
    async fn new(spawner: Spawner, peri: AppPeripherals) -> Result<&'static Self, Error> {
        let driver = DriverModule::new(peri.driver_peri).await?;
        for i in 0.. {
            for _ in 0..100 {
                driver.count().ok();
            }
            info!("{:?}", driver.count().ok());
            Timer::after_millis(1000).await;
        }
        let mut rng = RoscRng;
        let flash = FlashModule::new(peri.flash_peri).await?;
        let (radio, bt_device, net_device) = RadioModule::new(spawner, peri.radio_peri).await?;
        let ble = BleModule::new(spawner, bt_device).await?;
        let led = LedModule::new(spawner, radio).await?;
        let wifi = WifiModule::new(spawner, radio, net_device, &mut rng).await?;
        let mqtt = MqttModule::new(spawner, &wifi.stack()).await?;
        let state = flash.load().await?;
        let application = make_static!(Application {
            spawner,
            flash,
            ble,
            wifi,
            led,
            mqtt,
            driver,
            state: RefCell::new(state.clone()),
            wifi_status: Signal::new(),
            mqtt_status: Signal::new(),
            display_message: Signal::new(),
        });
        Ok(application)
    }
    fn initialize_settings(&'static self) -> Result<(), Error> {
        let state = self.state.borrow();
        let service = &self.ble.server().flappy_service;
        self.ble.set(&service.wifi_ssid, &state.wifi.ssid);
        self.ble.set(&service.wifi_password, &state.wifi.password);
        self.ble.set(&service.mqtt_hostname, &state.mqtt.hostname);
        self.ble
            .set(&service.mqtt_port, &format!("\"{}\"", &state.mqtt.port)?);
        self.ble.set(&service.mqtt_username, &state.mqtt.username);
        self.ble.set(&service.mqtt_password, &state.mqtt.password);
        self.ble.set(&service.mqtt_topic, &state.mqtt.topic);
        self.wifi.set_settings(state.wifi.clone());
        self.mqtt.set_settings(state.mqtt.clone());
        Ok(())
    }
    fn spawn_tasks(&'static self) -> Result<(), Error> {
        self.ble.start(self)?;
        self.mqtt.start(self)?;
        self.wifi.start(self)?;
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn notify_mqtt_status(application: &'static Application) {
                application.notify_mqtt_status().await;
            }
            notify_mqtt_status(self)?
        });
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn notify_wifi_status(application: &'static Application) {
                application.notify_wifi_status().await;
            }
            notify_wifi_status(self)?
        });
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn display_message(application: &'static Application) {
                application.display_message().await;
            }
            display_message(self)?
        });
        Ok(())
    }
    async fn notify_mqtt_status(&'static self) {
        loop {
            let status = self.mqtt_status.wait().await;
            self.ble
                .set_and_notify(
                    &self.ble.server().flappy_service.mqtt_status,
                    &format!("{}", status).unwrap_or_default(),
                )
                .await;
        }
    }
    async fn notify_wifi_status(&'static self) {
        loop {
            let status = self.wifi_status.wait().await;

            self.ble
                .set_and_notify(
                    &self.ble.server().flappy_service.wifi_status,
                    &format!("{}", status).unwrap_or_default(),
                )
                .await;
        }
    }
    async fn display_message(&'static self) {
        let mut display = Display::new(self.driver);
        if let Err(e) = display.run("").await {
            error!("{MODULE} error when resetting flaps: {}", e);
        }
        loop {
            let request = self.display_message.wait().await;
            if let Err(e) = display.run(&request.msg).await {
                error!("{MODULE} error when displaying message: {:?}", e);
            }
        }
    }
}

#[embassy_executor::task]
pub async fn main_task(spawner: Spawner, ap: AppPeripherals) {
    if let Result::<(), Error>::Err(e) = try {
        info!("{MODULE} Welcome to the 3D printed Split Flap Display!");
        info!(
            "{MODULE} GIT_VERSION: {}",
            built_info::GIT_VERSION.unwrap_or("<unknown>")
        );
        info!(
            "{MODULE} GIT_DIRTY: {}",
            built_info::GIT_DIRTY.unwrap_or(false)
        );
        info!(
            "{MODULE} GIT_HEAD_REF: {}",
            built_info::GIT_HEAD_REF.unwrap_or("<unknown>")
        );
        if let Some(sn) = serial_number() {
            info!("{MODULE} MCU Serial Number: {}", sn);
        }

        let app = Application::new(spawner, ap).await?;
        app.initialize_settings()?;
        app.spawn_tasks()?;
        pending::<!>().await;
    } {
        error!("{MODULE} Uncaught error: {:?}", e);
    }
}

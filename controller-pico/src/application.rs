use crate::ble::BleHandler;
use crate::display::Display;
use crate::error::Error;
use crate::flash::FlashSettings;
use crate::mqtt::{MqttHandler, MqttStatus};
use crate::peripherals::AppPeripherals;
use crate::root::{RootModule, RootModuleBuilder};
use crate::wifi::{WifiHandler, WifiStatus};
use core::cell::RefCell;
use core::future::pending;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use heapless::String;
use log::{error, info};
use serde::{Deserialize, Serialize};
use static_cell::StaticCell;

pub struct Application {
    root: &'static RootModule,
    state: RefCell<FlashSettings>,
    wifi_status: Signal<NoopRawMutex, WifiStatus>,
    mqtt_status: Signal<NoopRawMutex, MqttStatus>,
    display_message: Signal<NoopRawMutex, MqttRequest>,
}

#[derive(Serialize, Deserialize)]
struct MqttRequest {
    msg: String<128>,
}

impl MqttHandler for Application {
    fn handle_status(&self, status: MqttStatus) {
        self.mqtt_status.signal(status);
    }

    fn handle(&self, topic: &str, message: &[u8]) {
        if let Ok(message) = str::from_utf8(message) {
            info!("[ROOT] Received topic {} message {}", topic, message);
        }
        match serde_json_core::from_slice::<MqttRequest>(message) {
            Ok((message, _)) => {
                self.display_message.signal(message);
            }
            Err(e) => error!("Cannot parse message {:?}", e),
        }
    }
}

fn trim_null<const N: usize>(mut x: heapless::String<N>) -> heapless::String<N> {
    if x.as_bytes().last() == Some(&b'\0') {
        x.pop();
    }
    x
}

impl BleHandler for Application {
    fn handle_write(&self, id: u16) {
        let service = &self.root.ble.server().flappy_service;
        let ref mut state = *self.state.borrow_mut();
        let mut updated = false;
        if id == service.wifi_password.handle {
            updated = true;
            state.wifi.ssid = trim_null(self.root.ble.get(&service.wifi_ssid).unwrap_or_default());
            state.wifi.password = trim_null(
                self.root
                    .ble
                    .get(&service.wifi_password)
                    .unwrap_or_default(),
            );
            self.root.wifi.set_settings(state.wifi.clone());
        } else if id == service.mqtt_topic.handle {
            updated = true;
            state.mqtt.hostname = trim_null(
                self.root
                    .ble
                    .get(&service.mqtt_hostname)
                    .unwrap_or_default(),
            );
            let port = trim_null(self.root.ble.get(&service.mqtt_port).unwrap_or_default());
            let port = &port;
            let port = port.strip_prefix("\"").unwrap_or(port);
            let port = port.strip_suffix("\"").unwrap_or(port);
            let port: u16 = port.parse().unwrap_or_default();
            state.mqtt.port = port;
            state.mqtt.username = trim_null(
                self.root
                    .ble
                    .get(&service.mqtt_username)
                    .unwrap_or_default(),
            );
            state.mqtt.password = trim_null(
                self.root
                    .ble
                    .get(&service.mqtt_password)
                    .unwrap_or_default(),
            );
            state.mqtt.topic =
                trim_null(self.root.ble.get(&service.mqtt_topic).unwrap_or_default());
            self.root.mqtt.set_settings(state.mqtt.clone());
        }
        info!("new state = {:?}", state);
        if updated {
            if let Err(e) = self.root.flash.save(state) {
                error!("[ROOT] failed to update wifi settings in flash {}", e);
            }
        }
    }
}

impl WifiHandler for Application {
    fn handle_status(&self, status: WifiStatus) {
        self.wifi_status.signal(status);
    }
}

#[embassy_executor::task]
async fn notify_mqtt_status(application: &'static Application) {
    loop {
        let status = application.mqtt_status.wait().await;
        let mut formatted = heapless::String::new();
        use core::fmt::Write;
        write!(&mut formatted, "{}", status).ok();
        application
            .root
            .ble
            .set_and_notify(
                &application.root.ble.server().flappy_service.mqtt_status,
                &formatted,
            )
            .await;
    }
}

#[embassy_executor::task]
async fn notify_wifi_status(application: &'static Application) {
    loop {
        let status = application.wifi_status.wait().await;
        let mut formatted = heapless::String::new();
        use core::fmt::Write;
        write!(&mut formatted, "{}", status).ok();
        application
            .root
            .ble
            .set_and_notify(
                &application.root.ble.server().flappy_service.wifi_status,
                &formatted,
            )
            .await;
    }
}

#[embassy_executor::task]
async fn display_message(application: &'static Application) {
    let mut display = Display::new(application.root.driver);
    if let Err(e) = display.run("").await {
        error!("empty message flap error: {}", e);
    }
    loop {
        let request = application.display_message.wait().await;
        if let Err(e) = display.run(&request.msg).await {
            error!("{:?}", e);
        }
    }
}

#[embassy_executor::task]
pub async fn main_task(spawner: Spawner, ap: AppPeripherals) {
    if let Err(e) = main_impl(spawner, ap).await {
        error!("Uncaught error: {:?}", e);
    }
}

async fn main_impl(spawner: Spawner, ap: AppPeripherals) -> Result<(), Error> {
    for i in 0.. {
        info!("i = {}", i);
        Timer::after_secs(1).await;
    }
    let (root_task, root) = RootModuleBuilder { spawner }.build().await?;
    let state = root.flash.load().await?;
    static APPLICATION: StaticCell<Application> = StaticCell::new();
    let application = APPLICATION.init(Application {
        root,
        state: RefCell::new(state.clone()),
        wifi_status: Signal::new(),
        mqtt_status: Signal::new(),
        display_message: Signal::new(),
    });
    spawner.spawn(notify_mqtt_status(application)?);
    spawner.spawn(notify_wifi_status(application)?);
    info!("state = {:?}", state);
    let service = &root.ble.server().flappy_service;
    root.ble.set(&service.wifi_ssid, &state.wifi.ssid);
    root.ble.set(&service.wifi_password, &state.wifi.password);
    root.ble.set(&service.mqtt_hostname, &state.mqtt.hostname);
    root.ble.set(&service.mqtt_port, &{
        use core::fmt::Write;
        let mut s = heapless::String::new();
        write!(&mut s, "\"{}\"", &state.mqtt.port).ok();
        s
    });
    root.ble.set(&service.mqtt_username, &state.mqtt.username);
    root.ble.set(&service.mqtt_password, &state.mqtt.password);
    root.ble.set(&service.mqtt_topic, &state.mqtt.topic);
    root.wifi.set_settings(state.wifi);
    root.mqtt.set_settings(state.mqtt);
    root_task.spawn(spawner, application)?;
    spawner.spawn(display_message(application)?);
    pending::<!>().await;
}

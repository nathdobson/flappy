//! This example shows how to use USB (Universal Serial Bus) in the RP2040 chip.
//!
//! This creates the possibility to send log::info/warn/error/debug! to USB serial port.
#![deny(unused_must_use)]
#![allow(
    unused_variables,
    unused_mut,
    dead_code,
    internal_features,
    unused_imports
)]
#![no_std]
#![no_main]
#![feature(core_intrinsics)]
#![feature(future_join)]
#![feature(type_alias_impl_trait)]
#![feature(never_type)]
#![feature(try_blocks)]

use crate::ble::{BleHandler, BleModule, BleModuleBuilder, BleTask};
use crate::error::Error;
use crate::flash::{FlashModule, FlashModuleBuilder, FlashSettings};
use crate::led::{LedModule, LedModuleBuilder};
use crate::mqtt::{MqttHandler, MqttSettings, MqttStatus};
use crate::psram::PsramModuleBuilder;
use crate::radio::{RadioModule, RadioModuleBuilder};
use crate::root::{RootModule, RootModuleBuilder};
use crate::usb::{UsbModule, UsbModuleBuilder};
use crate::wifi::{WifiHandler, WifiModule, WifiModuleBuilder, WifiSettings, WifiStatus};
use core::cell::RefCell;
use core::future::pending;
use core::intrinsics::catch_unwind;
use core::str::from_utf8;
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_net::dns::{DnsQueryType, DnsSocket};
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::tcp::TcpSocket;
use embassy_rp::clocks::RoscRng;
use embassy_rp::flash::Async;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use embedded_tls::{
    Aes128GcmSha256, Aes256GcmSha384, NoVerify, TlsConfig, TlsConnection, TlsContext,
};
use log::{error, info};
use rust_mqtt::client::client::MqttClient;
use rust_mqtt::client::client_config::ClientConfig;
use rust_mqtt::packet::v5::publish_packet::QualityOfService;
use rust_mqtt::packet::v5::reason_codes::ReasonCode;
use rust_mqtt::utils::rng_generator::CountingRng;
use serde::Deserialize;
use serde_json_core::from_slice;
use static_cell::StaticCell;
use trouble_host::prelude::HeaplessString;

mod ble;
mod ble_gatt;
mod error;
mod flash;
mod led;
mod mqtt;
mod psram;
mod radio;
mod root;
mod runtime;
mod usb;
mod wifi;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    if let Err(e) = main_impl(spawner).await {
        error!("Uncaught error: {:?}", e);
    }
}

pub struct Application {
    root: &'static RootModule,
    state: RefCell<FlashSettings>,
    wifi_status: Signal<NoopRawMutex, WifiStatus>,
    mqtt_status: Signal<NoopRawMutex, MqttStatus>,
}

impl MqttHandler for Application {
    fn handle_status(&self, status: MqttStatus) {
        self.mqtt_status.signal(status);
    }

    fn handle(&self, topic: &str, message: &[u8]) {
        if let Ok(message) = str::from_utf8(message) {
            info!("[ROOT] Received topic {} message {}", topic, message);
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

async fn main_impl(spawner: Spawner) -> Result<(), Error> {
    let (root_task, root) = RootModuleBuilder { spawner }.build().await?;
    Timer::after(Duration::from_secs(5)).await;
    let state = root.flash.load().await?;
    static APPLICATION: StaticCell<Application> = StaticCell::new();
    let application = APPLICATION.init(Application {
        root,
        state: RefCell::new(state.clone()),
        wifi_status: Signal::new(),
        mqtt_status: Signal::new(),
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
    loop {
        info!("Heartbeat");
        Timer::after(Duration::from_secs(5)).await;
    }
}

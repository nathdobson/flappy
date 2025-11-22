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
use crate::mqtt::{MqttHandler, MqttSettings};
use crate::psram::PsramModuleBuilder;
use crate::radio::{RadioModule, RadioModuleBuilder};
use crate::root::{RootModule, RootModuleBuilder};
use crate::secrets::{MQTT_PASSWORD, MQTT_USERNAME};
use crate::usb::{UsbModule, UsbModuleBuilder};
use crate::wifi::{WifiHandler, WifiModule, WifiModuleBuilder};
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
mod secrets;
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
}

impl MqttHandler for Application {
    fn handle(&self, topic: &str, message: &[u8]) {
        if let Ok(message) = str::from_utf8(message) {
            info!("[ROOT] Received topic {} message {}", topic, message);
        }
    }
}

impl BleHandler for Application {
    fn handle_write(&self, id: u16) {
        info!("[ROOT] Received BLE write {}", id);
    }
}

impl WifiHandler for Application {
    fn handle_link_status(&self, state: bool) {
        if state {
            info!("[ROOT] Link up");
        } else {
            info!("[ROOT] Link down");
        }
    }

    fn handle_dhcp_status(&self, state: bool) {
        if state {
            info!("[ROOT] DHCP up");
        } else {
            info!("[ROOT] DHCP down");
        }
    }
}

async fn main_impl(spawner: Spawner) -> Result<(), Error> {
    let (root_task, root) = RootModuleBuilder { spawner }.build().await?;
    static APPLICATION: StaticCell<Application> = StaticCell::new();
    let application = APPLICATION.init(Application { root });
    root_task.spawn(spawner, application)?;
    root.mqtt.set_settings(MqttSettings {
        hostname: "u8c6afc1.ala.us-east-1.emqxsl.com".try_into().unwrap(),
        port: 8883,
        username: MQTT_USERNAME.try_into().unwrap(),
        password: MQTT_PASSWORD.try_into().unwrap(),
        topic: "testtopic/#".try_into().unwrap(),
    });
    loop {
        info!("Heartbeat");
        Timer::after(Duration::from_secs(5)).await;
    }
}

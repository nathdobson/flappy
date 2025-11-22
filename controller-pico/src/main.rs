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

use crate::ble::{BleModule, BleModuleBuilder, BleTask};
use crate::error::Error;
use crate::flash::{FlashModule, FlashModuleBuilder, FlashSettings};
use crate::led::{LedModule, LedModuleBuilder};
use crate::psram::PsramModuleBuilder;
use crate::radio::{RadioModule, RadioModuleBuilder};
use crate::root::RootModuleBuilder;
use crate::secrets::{MQTT_PASSWORD, MQTT_USERNAME};
use crate::usb::{UsbModule, UsbModuleBuilder};
use crate::wifi::{WifiModule, WifiModuleBuilder};
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
async fn main_impl(spawner: Spawner) -> Result<(), Error> {
    let (root_task, root) = RootModuleBuilder {}.build(spawner).await?;
    root_task.spawn(spawner)?;
    loop {
        info!("Hello");
        Timer::after(Duration::from_secs(1)).await;
    }
    // let mut rx_buffer = [0; 4096];
    // let mut tx_buffer = [0; 4096];
    // loop {
    //     let mut socket = TcpSocket::new(root.wifi.stack, &mut rx_buffer, &mut tx_buffer);
    //     // socket.set_timeout(Some(Duration::from_secs(10)));
    //     let dns = "u8c6afc1.ala.us-east-1.emqxsl.com";
    //     let port = 8883;
    //     let address = root.wifi.stack.dns_query(dns, DnsQueryType::A).await?[0];
    //
    //     let remote_endpoint = (address, port);
    //     info!("[MQTT] Connecting to address {:?}", remote_endpoint);
    //     socket.connect(remote_endpoint).await?;
    //     info!("[MQTT] Connected to TCP");
    //
    //     let mut read_record_buffer = [0; 16384];
    //     let mut write_record_buffer = [0; 16384];
    //     let config = TlsConfig::<Aes128GcmSha256>::new()
    //         .with_server_name(dns)
    //         .enable_rsa_signatures();
    //     let mut tls = TlsConnection::new(socket, &mut read_record_buffer, &mut write_record_buffer);
    //
    //     tls.open::<_, NoVerify>(TlsContext::new(&config, &mut RoscRng))
    //         .await?;
    //     info!("[MQTT] Connected to TLS");
    //
    //     let mut config = ClientConfig::new(
    //         rust_mqtt::client::client_config::MqttVersion::MQTTv5,
    //         CountingRng(20000),
    //     );
    //     // config.add_max_subscribe_qos(QualityOfService::QoS1);
    //     config.add_client_id("flappy");
    //     config.max_packet_size = 100;
    //     config.add_username(MQTT_USERNAME);
    //     config.add_password(MQTT_PASSWORD);
    //     let mut recv_buffer = [0; 80];
    //     let mut write_buffer = [0; 80];
    //
    //     let mut client =
    //         MqttClient::<_, 5, _>::new(tls, &mut write_buffer, 80, &mut recv_buffer, 80, config);
    //
    //     client.connect_to_broker().await?;
    //     info!("[MQTT] Connected to MQTT Server");
    //
    //     client.subscribe_to_topic("testtopic/#").await?;
    //
    //     loop {
    //         let (a, b) = client.receive_message().await?;
    //         info!("[MQTT] a={}", a);
    //         yield_now().await;
    //         info!("[MQTT] b={:?}", b);
    //     }
    //
    //     Timer::after(Duration::from_secs(10000)).await;
    // }
    pending::<!>().await;
    // let state = root.flash.load().await?;
    // let flappy_service = &root.ble.server().flappy_service;
    // root.ble.set(&flappy_service.wifi_ssid, &state.wifi_ssid);
    // root.ble
    //     .set(&flappy_service.wifi_password, &state.wifi_password);
    //
    // loop {
    //     let service = &root.ble.server().flappy_service;
    //     root.ble.listen(&service.wifi_password).await;
    //     let mut state = FlashState::default();
    //     state.wifi_ssid = root.ble.get(&flappy_service.wifi_ssid)?;
    //     state.wifi_password = root.ble.get(&flappy_service.wifi_password)?;
    //     root.flash.save(&state)?;
    //     info!("Updated flash");
    //     Timer::after(Duration::from_secs(1)).await;
    //     info!("New state: {:?}", root.flash.load().await?);
    //
    //     Timer::after(Duration::from_secs(1)).await;
    // }
}

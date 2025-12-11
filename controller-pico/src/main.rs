#![no_std]
#![no_main]
#![deny(unused_must_use)]
#![allow(
    unused_variables,
    unused_mut,
    dead_code,
    internal_features,
    unused_imports
)]
#![feature(core_intrinsics)]
#![feature(future_join)]
#![feature(type_alias_impl_trait)]
#![feature(never_type)]
#![feature(try_blocks)]
#![feature(debug_closure_helpers)]
#![allow(unreachable_code)]

use crate::application::main_task;
use crate::ble::BleModule;
use crate::display::Display;
use crate::error::Error;
use crate::flash::{FlashModule, FlashSettings};
use crate::led::LedModule;
use crate::mqtt::{MqttSettings, MqttStatus};
use crate::peripherals::build_peripherals;
use crate::radio::RadioModule;
use crate::wifi::{WifiHandler, WifiModule, WifiSettings, WifiStatus};
use core::cell::RefCell;
use core::future::pending;
use core::intrinsics::catch_unwind;
use core::str::from_utf8;
use cortex_m_rt::entry;
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_net::dns::{DnsQueryType, DnsSocket};
use embassy_net::tcp::TcpSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_rp::clocks::RoscRng;
use embassy_rp::flash::Async;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use embedded_tls::{
    Aes128GcmSha256, Aes256GcmSha384, NoVerify, TlsConfig, TlsConnection, TlsContext,
};
use heapless::String;
use log::{error, info};
// use rust_mqtt::client::client::MqttClient;
// use rust_mqtt::client::client_config::ClientConfig;
// use rust_mqtt::packet::v5::publish_packet::QualityOfService;
// use rust_mqtt::packet::v5::reason_codes::ReasonCode;
// use rust_mqtt::utils::rng_generator::CountingRng;
use serde::{Deserialize, Serialize};
use serde_json_core::from_slice;
use trouble_host::prelude::HeaplessString;

mod application;
mod ble;
mod ble_gatt;
mod display;
mod driver;
mod error;
mod executor;
mod flash;
mod global_alloc;
mod led;
mod mqtt;
mod peripherals;
mod product;
mod radio;
mod runtime;
mod wifi;

extern crate alloc;

#[entry]
unsafe fn main() -> ! {
    let (rp, ap) = build_peripherals();
    executor::run_program(
        move |spawner| runtime::runtime(spawner, rp),
        move |spawner| spawner.spawn(main_task(spawner, ap).unwrap()),
    );
}

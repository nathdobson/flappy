//! This example shows how to use USB (Universal Serial Bus) in the RP2040 chip.
//!
//! This creates the possibility to send log::info/warn/error/debug! to USB serial port.
#![deny(unused_must_use)]
#![allow(unused_variables, unused_mut, dead_code, internal_features)]
#![no_std]
#![no_main]
#![feature(core_intrinsics)]

use crate::ble::BleModuleBuilder;
use crate::radio::RadioModuleBuilder;
use crate::usb::UsbModuleBuilder;
use embassy_executor::Spawner;
use embassy_rp::clocks::RoscRng;
use embassy_time::{Duration, Timer};
use log::info;

mod ble;
mod radio;
mod runtime;
mod secrets;
mod usb;
mod wifi;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let usb_module = UsbModuleBuilder {
        spawner,
        usb: p.USB,
    }
    .build();
    info!("Starting...");
    Timer::after(Duration::from_secs(3)).await;
    info!("Started");

    let mut rng = RoscRng;
    let radio = RadioModuleBuilder {
        spawner,
        pin23: p.PIN_23,
        pin24: p.PIN_24,
        pin25: p.PIN_25,
        pin29: p.PIN_29,
        pio0: p.PIO0,
        dma_ch0: p.DMA_CH0,
    }
    .build()
    .await;
    let ble = BleModuleBuilder {
        bt_device: radio.bt_device,
    }
    .build()
    .await;

    loop {
        Timer::after(Duration::from_secs(5)).await;
        info!("Tick2");
    }
}

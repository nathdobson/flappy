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

use crate::ble::BleModuleBuilder;
use crate::error::Error;
use crate::led::LedModuleBuilder;
use crate::psram::PsramModuleBuilder;
use crate::radio::RadioModuleBuilder;
use crate::usb::UsbModuleBuilder;
use core::future::pending;
use core::intrinsics::catch_unwind;
use embassy_executor::Spawner;
use embassy_rp::clocks::RoscRng;
use embassy_time::{Duration, Timer};
use log::{error, info};

mod ble;
mod error;
mod led;
mod ble_gatt;
mod psram;
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
    Timer::after(Duration::from_secs(5)).await;
    info!("Started");
    Timer::after(Duration::from_secs(2)).await;

    let foo: Result<(), Error> = try {
        let mut rng = RoscRng;
        let mut radio = RadioModuleBuilder {
            spawner,
            pin23: p.PIN_23,
            pin24: p.PIN_24,
            pin25: p.PIN_25,
            pin29: p.PIN_29,
            pio0: p.PIO0,
            dma_ch0: p.DMA_CH0,
        }
        .build()
        .await?;
        let ble = BleModuleBuilder {
            spawner,
            bt_device: radio.bt_device,
        }
        .build()
        .await;

        let led = LedModuleBuilder {
            spawner,
            control: radio.control,
        }
        .build();
    };
    if let Err(e) = foo {
        error!("Uncaught error: {}", e);
    };
    pending::<()>().await;
}

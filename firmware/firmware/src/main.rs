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
#![feature(never_type)]
#![feature(try_blocks)]
#![feature(str_split_remainder)]
#![feature(allocator_api)]
#![feature(c_size_t)]
#![allow(unreachable_code)]

use core::future::pending;
use crate::application::main_task;
use crate::peripherals::build_peripherals;
use crate::runtime::RuntimeModule;
use cortex_m_rt::entry;
mod application;
#[cfg(feature = "ble")]
mod ble;
mod cli;
#[cfg(feature = "display")]
mod controller;
mod display_proto;
#[cfg(feature = "display")]
mod driver;
mod error;
mod executor;
#[cfg(feature = "flash")]
mod flash;
mod global_alloc;
#[cfg(feature = "radio")]
mod led;
#[cfg(feature = "mqtt")]
mod mqtt;
mod peripherals;
mod product;
#[cfg(feature = "radio")]
mod radio;
mod runtime;
mod settings_channel;
#[cfg(feature = "usb")]
mod usb;
#[cfg(feature = "usb")]
mod usb_reset;
#[cfg(feature = "usb")]
mod usb_serial;
#[cfg(all(feature="usb",feature = "setup"))]
mod usb_setup;
#[cfg(feature = "wifi")]
mod wifi;
mod make_static;
#[cfg(feature = "spindle")]
mod spindle;
mod display;

extern crate alloc;

#[entry]
unsafe fn main() -> ! {
    let (rp, ap) = build_peripherals();
    executor::run_program(
        move |spawner| RuntimeModule::new(spawner, rp),
        move |spawner, runtime| spawner.spawn(main_task(spawner, runtime, ap).unwrap()),
    );
}


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
#![allow(unreachable_code)]
#![allow(unused_features)]
#![feature(macro_derive)]
#![feature(type_alias_impl_trait)]

use crate::application::main_task;
use crate::peripherals::build_peripherals;
use crate::runtime::RuntimeModule;
use ::runtime::RemoteSpawner;
use core::future::pending;
use cortex_m_rt::entry;
use embassy_executor::Executor;
use make_static::make_static;

mod application;
#[cfg(feature = "ble")]
mod ble;
mod cli;
#[cfg(feature = "display")]
mod controller;
#[cfg(feature = "display")]
mod driver;
mod error;
// mod executor;
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
// #[cfg(feature = "usb")]
// mod usb;
// #[cfg(feature = "usb")]
// mod usb_reset;
// #[cfg(feature = "usb")]
// mod usb_serial;
// #[cfg(all(feature="usb",feature = "setup"))]
// mod usb_setup;
mod bootsel;
mod display;
mod interrupts;
#[cfg(feature = "spindle")]
mod spindle;
#[cfg(feature = "usb")]
mod usb;
#[cfg(feature = "wifi")]
mod wifi;

extern crate alloc;

#[entry]
unsafe fn main() -> ! {
    ::runtime::start_runtime(|runtime| {
        let (rp, ap) = build_peripherals();
        let runtime_module = RuntimeModule::new(runtime.interrupt, rp);
        runtime
            .thread
            .spawn(main_task(runtime.thread, runtime_module, ap).unwrap());
    })
}

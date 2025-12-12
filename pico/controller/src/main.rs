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
use crate::peripherals::build_peripherals;
use crate::runtime::RuntimeModule;
use cortex_m_rt::entry;
mod application;
#[cfg(feature = "radio")]
mod ble;
#[cfg(feature = "radio")]
mod ble_gatt;
#[cfg(feature = "display")]
mod display;
#[cfg(feature = "display")]
mod driver;
mod error;
mod executor;
#[cfg(feature = "flash")]
mod flash;
mod flash_proto;
mod global_alloc;
#[cfg(feature = "radio")]
mod led;
#[cfg(feature = "radio")]
mod mqtt;
mod mqtt_proto;
mod peripherals;
mod product;
#[cfg(feature = "radio")]
mod radio;
mod runtime;
#[cfg(feature = "radio")]
mod wifi;
mod wifi_proto;

extern crate alloc;

#[entry]
unsafe fn main() -> ! {
    let (rp, ap) = build_peripherals();
    executor::run_program(
        move |spawner| RuntimeModule::new(spawner, rp),
        move |spawner, runtime| spawner.spawn(main_task(spawner, runtime, ap).unwrap()),
    );
}

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
use crate::kernel::KernelModule;
use crate::peripherals::build_peripherals;
use ::runtime::RemoteSpawn;
use core::future::pending;
use cortex_m_rt::entry;
use dummy_alloc::DummyAllocator;
use embassy_executor::Executor;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, PIO0};
use log::error;
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
mod kernel;
#[cfg(feature = "radio")]
mod led;
#[cfg(feature = "mqtt")]
mod mqtt;
mod peripherals;
#[cfg(feature = "radio")]
mod radio;
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
#[cfg(feature = "spindle")]
mod spindle;
#[cfg(feature = "usb")]
mod usb;
#[cfg(feature = "wifi")]
mod wifi;

mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

extern crate alloc;

#[entry]
unsafe fn main() -> ! {
    #[cfg(feature = "heap")]
    unsafe {
        embedded_alloc::init!(HEAP, 16384);
    }
    ::runtime::start_runtime(|runtime| {
        let (kernel_peri, app_peri) = build_peripherals();
        let kernel = KernelModule::new(runtime.interrupt, kernel_peri);
        runtime
            .thread
            .spawn(main_task(runtime.thread, kernel, app_peri).unwrap());
    })
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info);
    loop {}
}

#[cfg(not(feature = "heap"))]
#[global_allocator]
pub static EMPTY_ALLOCATOR: DummyAllocator = DummyAllocator;

#[cfg(feature = "heap")]
#[global_allocator]
static HEAP: embedded_alloc::LlffHeap = embedded_alloc::LlffHeap::empty();

bind_interrupts!(pub struct Irqs {
    DMA_IRQ_0 => embassy_rp::dma::InterruptHandler<DMA_CH0>, embassy_rp::dma::InterruptHandler<DMA_CH1>;
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
});

use crate::error::Error;
use crate::product;
use crate::usb::{FlappyUsbServer, UsbModule};
use ::runtime::RemoteSpawn;
use core::fmt::Arguments;
use core::intrinsics::abort;
use core::{fmt, mem};
use embassy_executor::{SendSpawner, Spawner};
use embassy_futures::join::join;
use embassy_futures::select::{Either, select};
use embassy_rp::otp::get_chipid;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, Endpoint, In, Out};
use embassy_rp::{Peri, bind_interrupts, rom_data};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, DynamicReceiver};
use embassy_sync::mutex::Mutex;
use embassy_sync::pipe::Pipe;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Timer, block_for};
use heapless::{String, Vec};
use log::{Level, Log, Metadata, Record, error, info, set_logger, set_max_level};
use make_static::make_static;

const MODULE: &'static str = "[RUN  ]";

#[allow(non_snake_case)]
pub struct KernelPeripherals {
    pub USB: Peri<'static, USB>,
}

pub struct KernelModule {
    pub usb: &'static FlappyUsbServer,
}

impl KernelModule {
    pub fn new(spawner: SendSpawner, peri: KernelPeripherals) -> &'static Self {
        let module: &'static KernelModule = make_static!(
            KernelModule,
            KernelModule {
                usb: FlappyUsbServer::new(spawner, peri.USB),
            }
        );
        module
    }
}

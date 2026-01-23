use crate::error::Error;
use crate::product::serial_number;
use crate::{make_static, product};
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

const MODULE: &'static str = "[RUN  ]";

#[allow(non_snake_case)]
pub struct RuntimePeripherals {
    pub USB: Peri<'static, USB>,
}

pub struct RuntimeModule {
    #[cfg(feature = "usb")]
    pub usb: &'static crate::usb::UsbModule,
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info);
    loop {}
}

impl RuntimeModule {
    pub fn new(spawner: SendSpawner, peri: RuntimePeripherals) -> &'static Self {
        #[cfg(feature = "usb")]
        let usb = crate::usb::UsbModule::new();
        let module: &'static RuntimeModule = make_static!(
            RuntimeModule,
            RuntimeModule {
                #[cfg(feature = "usb")]
                usb
            }
        );

        spawner.spawn({
            #[embassy_executor::task]
            async fn start_task(module: &'static RuntimeModule, peri: RuntimePeripherals) {
                if let Err(e) = module.start(peri).await {
                    info!("uncaught runtime error: {:?}", e);
                }
            }
            start_task(module, peri).unwrap()
        });
        module
    }
    async fn start(&self, peri: RuntimePeripherals) -> Result<(), Error> {
        let spawner = unsafe { Spawner::for_current_executor().await };
        #[cfg(feature = "usb")]
        self.usb.start(spawner, peri.USB).await?;
        Ok(())
    }
}

pub fn reboot_to_bootsel() -> ! {
    rom_data::reboot(0x0002, 500, 0, 0);
    unreachable!()
}

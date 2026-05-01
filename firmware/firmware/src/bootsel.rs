use crate::error::Error;
use ::make_static::make_static;
use core::cell::{Cell, RefCell};
use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::peripherals::BOOTSEL;
use embassy_time::Delay;
use embassy_time::Instant;
use embedded_hal_async::delay::DelayNs;
use log::info;
use runtime::LocalSpawn;

pub struct BootselPeripherals {
    pub bootsel: Peri<'static, BOOTSEL>,
}
pub struct BootselModule {
    bootsel: RefCell<Peri<'static, BOOTSEL>>,
}

impl BootselModule {
    pub fn new(spawner: Spawner, mut peri: BootselPeripherals) -> Result<&'static Self, Error> {
        let module: &_ = make_static!(
            BootselModule,
            BootselModule {
                bootsel: RefCell::new(peri.bootsel)
            }
        );
        Ok(module)
    }
    pub fn is_pressed(&self) -> bool {
        let mut bootsel = self.bootsel.borrow_mut();
        let bootsel = bootsel.reborrow();
        let bootsel = embassy_rp::bootsel::is_bootsel_pressed(bootsel);
        bootsel
    }
}

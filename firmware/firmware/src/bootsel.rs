use crate::error::Error;
use ::make_static::make_static;
use core::cell::Cell;
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
    pub pressed: Cell<bool>,
}

impl BootselModule {
    pub fn new(spawner: Spawner, mut peri: BootselPeripherals) -> Result<&'static Self, Error> {
        let module: &_ = make_static!(
            BootselModule,
            BootselModule {
                pressed: Cell::new(false)
            }
        );
        make_static!(_, LocalSpawn::new(spawner)).spawn(move || async move {
            loop {
                let bootsel = peri.bootsel.reborrow();
                let bootsel = embassy_rp::bootsel::is_bootsel_pressed(bootsel);
                module.pressed.replace(bootsel);
                Delay.delay_ms(100).await;
            }
        });
        Ok(module)
    }
    pub fn is_pressed(&self) -> bool {
        self.pressed.get()
    }
}

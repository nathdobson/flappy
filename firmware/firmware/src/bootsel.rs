use crate::error::Error;
use core::cell::Cell;
use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::peripherals::BOOTSEL;
use embassy_time::Delay;
use embassy_time::Instant;
use embedded_hal_async::delay::DelayNs;
use log::info;
use ::make_static::make_static;
pub struct BootselPeripherals {
    pub bootsel: Peri<'static, BOOTSEL>,
}
pub struct BootselModule {
    pub pressed: Cell<bool>,
}

impl BootselModule {
    pub fn new(spawner: Spawner, peri: BootselPeripherals) -> Result<&'static Self, Error> {
        let module = make_static!(
            BootselModule,
            BootselModule {
                pressed: Cell::new(false)
            }
        );
        spawner.spawn({
            #[embassy_executor::task]
            async fn poll_pressed(
                module: &'static BootselModule,
                mut bootsel: Peri<'static, BOOTSEL>,
            ) {
                loop {
                    let bootsel = bootsel.reborrow();
                    let bootsel = embassy_rp::bootsel::is_bootsel_pressed(bootsel);
                    module.pressed.replace(bootsel);
                    Delay.delay_ms(100).await;
                }
            }
            poll_pressed(module, peri.bootsel)?
        });
        Ok(module)
    }
    pub fn is_pressed(&self) -> bool {
        self.pressed.get()
    }
}

use crate::error::Error;
use core::cell::RefCell;
use cortex_m::asm::delay;
use embassy_futures::yield_now;
use embassy_rp::gpio::{Level, Output, SlewRate};
use embassy_rp::peripherals::{PIN_0, PIN_1, PIN_2, PIN_3, PIN_4, PIN_5, PIN_6, SPI0};
use embassy_rp::pwm::Pwm;
use embassy_rp::spi::{Blocking, Spi};
use embassy_rp::{Peri, spi};
use embassy_time::{Delay, Timer};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use log::{error, info};
use error_report::Report;
use make_static::make_static;

const MODULE: &str = "[DRIVE]";

#[allow(non_snake_case)]
pub struct DriverPeripherals {
    /// CIPO
    pub PIN_0: Peri<'static, PIN_0>,
    /// LATCH
    pub PIN_1: Peri<'static, PIN_1>,
    /// GND
    pub GND1: (),
    /// CLOCK
    pub PIN_2: Peri<'static, PIN_2>,
    /// COPI
    pub PIN_3: Peri<'static, PIN_3>,
    /// LOAD
    pub PIN_4: Peri<'static, PIN_4>,
    /// RESET or FAULT
    pub PIN_5: Peri<'static, PIN_5>,
    /// GND
    pub GND2: (),
    /// ENABLE
    pub PIN_6: Peri<'static, PIN_6>,
    /// SPI
    pub SPI0: Peri<'static, SPI0>,
}
struct DriverInner {
    spi: Spi<'static, SPI0, Blocking>,
    load: Output<'static>,
    latch: Output<'static>,
    enable: Output<'static>,
}
pub struct DriverModule {
    inner: RefCell<DriverInner>,
}

impl DriverModule {
    pub fn set_enabled(&self, enabled: bool) {
        let ref mut inner = *self.inner.borrow_mut();
        if enabled {
            inner.enable.set_low();
        } else {
            inner.enable.set_high();
        }
    }
    pub fn count(&self) -> Result<usize, Error> {
        let ref mut inner = *self.inner.borrow_mut();
        inner.load.set_low();
        Delay.delay_us(10);
        inner.load.set_high();
        Delay.delay_us(10);
        for count in 0usize..20 {
            let mut data = [0u8];
            inner.spi.blocking_read(&mut data)?;
            if data[0] == 0xFF {
                // info!("{MODULE} Counted {} flaps", count);
                return Ok(count);
            } else {
                if data[0] != 0b01 && data[0] != 0b11 {
                    // info!("Bad data {:?}", data);
                }
            }
        }
        Err(Error::CountFailure)
    }
    pub fn write(&self, data: &[u8]) -> Result<(), Error> {
        let ref mut inner = *self.inner.borrow_mut();
        inner.spi.blocking_write(data)?;
        inner.latch.set_low();
        Delay.delay_us(10);
        inner.latch.set_high();
        Delay.delay_us(10);
        Ok(())
    }
    pub fn read(&self, data: &mut [u8]) -> Result<(), Error> {
        let ref mut inner = *self.inner.borrow_mut();
        inner.load.set_low();
        Delay.delay_us(10);
        inner.load.set_high();
        Delay.delay_us(10);
        inner.spi.blocking_read(data)?;
        Ok(())
    }
}

impl DriverModule {
    pub async fn new(peri: DriverPeripherals) -> Result<&'static DriverModule, Error> {
        let mut config = spi::Config::default();
        config.frequency = 1000000;
        let mut spi = Spi::new_blocking(peri.SPI0, peri.PIN_2, peri.PIN_3, peri.PIN_0, config);
        let mut load = Output::new(peri.PIN_4, Level::High);
        let mut latch = Output::new(peri.PIN_1, Level::High);
        let mut enable = Output::new(peri.PIN_6, Level::High);

        let module = make_static!(
            DriverModule,
            DriverModule {
                inner: RefCell::new(DriverInner {
                    spi,
                    load,
                    latch,
                    enable,
                }),
            }
        );
        Ok(module)
    }
    pub async fn run_read_test(&self) {
        loop {
            let mut buf = [0u8; 20];
            if let Err(e) = self.read(&mut buf) {
                error!("error during read test: {}", Report::new(e));
                return;
            }
            info!("read = {:?}", buf);
            Timer::after_millis(100).await;
        }
    }
}

use crate::error::Error;
use core::cell::RefCell;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{PIN_0, PIN_1, PIN_2, PIN_3, PIN_4, PIN_5, PIN_6, SPI0};
use embassy_rp::spi::{Blocking, Spi};
use embassy_rp::{spi, Peri};
use embassy_time::{Delay, Timer};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use log::info;
use static_cell::StaticCell;
pub struct DriverBuilder {
    pub cipo: Peri<'static, PIN_0>,
    pub copi: Peri<'static, PIN_3>,
    pub clock: Peri<'static, PIN_2>,
    pub spi: Peri<'static, SPI0>,

    pub latch: Peri<'static, PIN_1>,
    pub load: Peri<'static, PIN_4>,
    pub reset: Peri<'static, PIN_5>,
    pub enable: Peri<'static, PIN_6>,
}

struct DriverInner {
    spi: Spi<'static, SPI0, Blocking>,
    load: Output<'static>,
    latch: Output<'static>,
    enable: Output<'static>,
    reset: Output<'static>,
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
        for count in 0usize..1000 {
            let mut data = [0u8];
            inner.spi.blocking_read(&mut data)?;
            if data[0] == 0xFF {
                info!("Counted {}", count);
                return Ok(count);
            }
        }
        Err(Error::StrError("count failure"))
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

impl DriverBuilder {
    pub async fn build(self) -> Result<&'static DriverModule, Error> {
        let mut config = spi::Config::default();
        config.frequency = 1000000;
        let mut spi = Spi::new_blocking(self.spi, self.clock, self.copi, self.cipo, config);
        let mut load = Output::new(self.load, Level::High);
        let mut latch = Output::new(self.latch, Level::High);
        let mut enable = Output::new(self.enable, Level::Low);
        let mut reset = Output::new(self.reset, Level::Low);
        // for i in 0.. {
        //     spi.blocking_write(&[if i % 2 == 0 { 0xFF } else { 0 }])?;
        //     Timer::after_micros(1000).await;
        //     latch.set_low();
        //     Timer::after_micros(1000).await;
        //     latch.set_high();
        //     Timer::after_secs(1).await;
        // }
        // for i in 0.. {
        //     load.set_low();
        //     Timer::after_micros(1000).await;
        //     load.set_high();
        //     Timer::after_micros(1000).await;
        //     let mut data = [0u8; 10];
        //     spi.blocking_read(&mut data)?;
        //     if i % 100 == 0 {
        //         info!("SPI read = {:?}", data);
        //     }
        // }
        static MODULE: StaticCell<DriverModule> = StaticCell::new();
        let module = MODULE.init(DriverModule {
            inner: RefCell::new(DriverInner {
                spi,
                load,
                latch,
                enable,
                reset,
            }),
        });
        Ok(module)
    }
}

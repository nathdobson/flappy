use crate::driver::DriverPeripherals;
use crate::flash::FlashPeripherals;
use crate::radio::RadioPeripherals;
use crate::runtime::RuntimePeripherals;
use embassy_rp::Peri;
use embassy_rp::peripherals::USB;

pub struct AppPeripherals {
    pub flash_peri: FlashPeripherals,
    pub driver_peri: DriverPeripherals,
    pub radio_peri: RadioPeripherals,
}

pub fn build_peripherals() -> (RuntimePeripherals, AppPeripherals) {
    let p = embassy_rp::init(Default::default());
    (
        RuntimePeripherals { USB: p.USB },
        AppPeripherals {
            flash_peri: FlashPeripherals {
                FLASH: p.FLASH,
                DMA_CH1: p.DMA_CH1,
            },
            driver_peri: DriverPeripherals {
                PIN_0: p.PIN_0,
                PIN_1: p.PIN_1,
                GND1: (),
                PIN_2: p.PIN_2,
                PIN_3: p.PIN_3,
                PIN_4: p.PIN_4,
                PIN_5: p.PIN_5,
                PIN_6: p.PIN_6,
                GND2: (),
                SPI0: p.SPI0,
            },
            radio_peri: RadioPeripherals {
                PIN_23: p.PIN_23,
                PIN_24: p.PIN_24,
                PIN_25: p.PIN_25,
                PIN_29: p.PIN_29,
                PIO0: p.PIO0,
                DMA_CH0: p.DMA_CH0,
            },
        },
    )
}

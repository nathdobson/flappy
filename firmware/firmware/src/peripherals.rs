use crate::bootsel::BootselPeripherals;
use crate::kernel::KernelPeripherals;
use embassy_rp::Peri;
use embassy_rp::peripherals::USB;

pub struct AppPeripherals {
    #[cfg(feature = "flash")]
    pub flash_peri: crate::flash::FlashPeripherals,
    #[cfg(feature = "display")]
    pub driver_peri: crate::driver::DriverPeripherals,
    #[cfg(feature = "radio")]
    pub radio_peri: radio_builder::RadioPeripherals,
    pub bootsel: BootselPeripherals,
}

pub fn build_peripherals() -> (KernelPeripherals, AppPeripherals) {
    let p = embassy_rp::init(Default::default());
    (
        KernelPeripherals { USB: p.USB },
        AppPeripherals {
            #[cfg(feature = "flash")]
            flash_peri: crate::flash::FlashPeripherals {
                FLASH: p.FLASH,
                DMA_CH1: p.DMA_CH1,
            },
            #[cfg(feature = "display")]
            driver_peri: crate::driver::DriverPeripherals {
                PIN_0: p.PIN_0,
                PIN_1: p.PIN_1,
                GND1: (),
                PIN_2: p.PIN_2,
                PIN_3: p.PIN_3,
                PIN_4: p.PIN_4,
                PIN_5: p.PIN_5,
                GND2: (),
                PIN_6: p.PIN_6,
                SPI0: p.SPI0,
            },
            #[cfg(feature = "radio")]
            radio_peri: radio_builder::RadioPeripherals {
                PIN_23: p.PIN_23,
                PIN_24: p.PIN_24,
                PIN_25: p.PIN_25,
                PIN_29: p.PIN_29,
                PIO0: p.PIO0,
                DMA_CH0: p.DMA_CH0,
            },
            bootsel: BootselPeripherals { bootsel: p.BOOTSEL },
        },
    )
}

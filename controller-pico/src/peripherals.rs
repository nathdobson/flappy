use embassy_rp::Peri;
use embassy_rp::peripherals::USB;
use crate::radio::RadioPeripherals;
use crate::runtime::RuntimePeripherals;

pub struct AppPeripherals {
    radio_peripherals: RadioPeripherals,
}

pub fn build_peripherals() -> (RuntimePeripherals, AppPeripherals) {
    let p = embassy_rp::init(Default::default());
    (
        RuntimePeripherals { USB: p.USB },
        AppPeripherals {
            radio_peripherals: RadioPeripherals {
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

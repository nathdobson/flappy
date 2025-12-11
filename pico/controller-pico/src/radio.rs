use crate::error::{Error, Result};
use core::cell::RefCell;
use cyw43::bluetooth::BtDriver;
use cyw43::{Control, NetDriver, Runner};
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_rp::dma::AnyChannel;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_rp::pio::Pio;
use embassy_rp::{Peri, bind_interrupts};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use log::info;
use static_cell::make_static;
const MODULE: &'static str = "[Radio]";

bind_interrupts!(struct PioIrqs {
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
});

type MyRunner = cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>;

#[allow(non_snake_case)]
pub struct RadioPeripherals {
    pub PIN_23: Peri<'static, PIN_23>,
    pub PIN_24: Peri<'static, PIN_24>,
    pub PIN_25: Peri<'static, PIN_25>,
    pub PIN_29: Peri<'static, PIN_29>,
    pub PIO0: Peri<'static, PIO0>,
    pub DMA_CH0: Peri<'static, DMA_CH0>,
}

pub struct RadioModule {
    pub control: Mutex<NoopRawMutex, Control<'static>>,
}

impl RadioModule {
    pub async fn new(
        spawner: Spawner,
        peri: RadioPeripherals,
    ) -> Result<(&'static RadioModule, BtDriver<'static>, NetDriver<'static>)> {
        info!("[Radio] Connecting to CYW43 radio transceiver over PIO-SPI");
        let pwr = Output::new(peri.PIN_23, Level::Low);
        let cs = Output::new(peri.PIN_25, Level::High);
        let mut pio = Pio::new(peri.PIO0, PioIrqs);
        let spi = PioSpi::new(
            &mut pio.common,
            pio.sm0,
            // SPI communication won't work if the speed is too high, so we use a divider larger than `DEFAULT_CLOCK_DIVIDER`.
            // See: https://github.com/embassy-rs/embassy/issues/3960.
            RM2_CLOCK_DIVIDER,
            pio.irq0,
            cs,
            peri.PIN_24,
            peri.PIN_29,
            peri.DMA_CH0,
        );

        let state = make_static!(cyw43::State::new());
        let (net_device, bt_device, mut control, runner) = cyw43::new_with_bluetooth(
            state,
            pwr,
            spi,
            cyw43_firmware::CYW43_43439A0,
            cyw43_firmware::CYW43_43439A0_BTFW,
        )
        .await;

        spawner.spawn({
            #[embassy_executor::task]
            async fn cyw43_task(runner: MyRunner) -> ! {
                runner.run().await
            }
            cyw43_task(runner)?
        });

        control.init(cyw43_firmware::CYW43_43439A0_CLM).await;
        control
            .set_power_management(cyw43::PowerManagementMode::None)
            .await;
        let module = make_static!(RadioModule {
            control: Mutex::new(control),
        });
        info!("{MODULE} Connected");
        yield_now().await;

        Ok((module, bt_device, net_device))
    }
}

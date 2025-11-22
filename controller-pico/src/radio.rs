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
use embassy_rp::{bind_interrupts, Peri};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use log::info;
use static_cell::StaticCell;

bind_interrupts!(struct PioIrqs {
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
});

type MyRunner = cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, AnyChannel>>;

#[embassy_executor::task]
async fn cyw43_task(runner: MyRunner) -> ! {
    runner.run().await
}

pub struct RadioModuleBuilder {
    pub spawner: Spawner,
    pub pin23: Peri<'static, PIN_23>,
    pub pin24: Peri<'static, PIN_24>,
    pub pin25: Peri<'static, PIN_25>,
    pub pin29: Peri<'static, PIN_29>,
    pub pio0: Peri<'static, PIO0>,
    pub dma_ch0: Peri<'static, AnyChannel>,
}

pub struct RadioModule {
    pub control: Mutex<NoopRawMutex, Control<'static>>,
}

pub struct RadioTask {}

impl RadioTask {
    pub fn spawn(self, spawner: Spawner) -> Result<()> {
        Ok(())
    }
}

impl RadioModuleBuilder {
    #[must_use]
    pub async fn build(
        self,
    ) -> Result<(
        RadioTask,
        BtDriver<'static>,
        NetDriver<'static>,
        &'static RadioModule,
    )> {
        info!("[Radio] starting");
        let pwr = Output::new(self.pin23, Level::Low);
        let cs = Output::new(self.pin25, Level::High);
        let mut pio = Pio::new(self.pio0, PioIrqs);
        let spi = PioSpi::new(
            &mut pio.common,
            pio.sm0,
            // SPI communication won't work if the speed is too high, so we use a divider larger than `DEFAULT_CLOCK_DIVIDER`.
            // See: https://github.com/embassy-rs/embassy/issues/3960.
            RM2_CLOCK_DIVIDER,
            pio.irq0,
            cs,
            self.pin24,
            self.pin29,
            self.dma_ch0,
        );

        static STATE: StaticCell<cyw43::State> = StaticCell::new();
        let state = STATE.init(cyw43::State::new());
        let (net_device, bt_device, mut control, runner) = cyw43::new_with_bluetooth(
            state,
            pwr,
            spi,
            cyw43_firmware::CYW43_43439A0,
            cyw43_firmware::CYW43_43439A0_BTFW,
        )
        .await;

        self.spawner.spawn(cyw43_task(runner)?);

        control.init(cyw43_firmware::CYW43_43439A0_CLM).await;
        control
            .set_power_management(cyw43::PowerManagementMode::None)
            .await;
        static MODULE: StaticCell<RadioModule> = StaticCell::new();
        let module = MODULE.init(RadioModule {
            control: Mutex::new(control),
        });
        info!("[Radio] started");
        yield_now().await;

        Ok((RadioTask {}, bt_device, net_device, module))
    }
}

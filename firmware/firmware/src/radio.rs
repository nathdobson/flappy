use crate::interrupts::Irqs;
use crate::make_static;
use core::mem;
use cyw43::{A4, Aligned, Control, NetDriver, aligned_bytes};
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use embassy_executor::{SpawnError, Spawner};
use embassy_futures::yield_now;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_rp::pio::{Common, Irq, IrqFlags, Pio, StateMachine};
use embassy_rp::{Peri, bind_interrupts, dma};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use fixed::FixedU32;
use log::info;

const MODULE: &'static str = "[Radio]";

type MyRunner = cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>;

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
    // Dropping this causes weird stuff to happen
    common: Common<'static, PIO0>,
    irq_flags: IrqFlags<'static, PIO0>,
    irq1: Irq<'static, PIO0, 1>,
    irq2: Irq<'static, PIO0, 2>,
    irq3: Irq<'static, PIO0, 3>,
    sm1: StateMachine<'static, PIO0, 1>,
    sm2: StateMachine<'static, PIO0, 2>,
    sm3: StateMachine<'static, PIO0, 3>,
}

pub struct RadioDrivers {
    pub module: &'static RadioModule,
    #[cfg(feature = "ble")]
    pub ble: cyw43::bluetooth::BtDriver<'static>,
    #[cfg(feature = "wifi")]
    pub net: NetDriver<'static>,
    pub mac_address: [u8; 6],
}

static FIRMWARE: &'static Aligned<A4, [u8]> =
    aligned_bytes!("../../../submodules/embassy/cyw43-firmware/43439A0.bin");
#[cfg(feature = "ble")]
static FIRMWARE_BTFW: &'static Aligned<A4, [u8]> =
    aligned_bytes!("../../../submodules/embassy/cyw43-firmware/43439A0_btfw.bin");
static FIRMWARE_CLM: &'static Aligned<A4, [u8]> =
    aligned_bytes!("../../../submodules/embassy/cyw43-firmware/43439A0_clm.bin");
static FIRMWARE_NVRAM: &'static Aligned<A4, [u8]> =
    aligned_bytes!("../../../submodules/embassy/cyw43-firmware/nvram_rp2040.bin");

impl RadioModule {
    pub async fn new(spawner: Spawner, peri: RadioPeripherals) -> Result<RadioDrivers, SpawnError> {
        let module: &'static mut RadioModule;
        info!("[Radio] Connecting to CYW43 radio transceiver over PIO-SPI");
        let pwr = Output::new(peri.PIN_23, Level::Low);
        let cs = Output::new(peri.PIN_25, Level::High);
        let mut pio = Pio::new(peri.PIO0, Irqs);
        let spi = PioSpi::new(
            &mut pio.common,
            pio.sm0,
            // SPI communication won't work if the speed is too high, so we use a divider larger than `DEFAULT_CLOCK_DIVIDER`.
            // See: https://github.com/embassy-rs/embassy/issues/3960.
            // This value seems to be pretty good to limit BLE corruption.
            #[cfg(feature = "ble")]
            FixedU32::from_bits(0x0B00),
            #[cfg(not(feature = "ble"))]
            RM2_CLOCK_DIVIDER,
            pio.irq0,
            cs,
            peri.PIN_24,
            peri.PIN_29,
            dma::Channel::new(peri.DMA_CH0, Irqs),
        );

        let state = make_static!(cyw43::State, cyw43::State::new());
        #[cfg(feature = "ble")]
        let (net_device, bt_device, mut control, runner) =
            cyw43::new_with_bluetooth(state, pwr, spi, FIRMWARE, FIRMWARE_BTFW, FIRMWARE_NVRAM)
                .await;
        #[cfg(not(feature = "ble"))]
        let (net_device, mut control, runner) =
            cyw43::new(state, pwr, spi, FIRMWARE, FIRMWARE_NVRAM).await;

        spawner.spawn({
            #[embassy_executor::task]
            async fn cyw43_task(runner: MyRunner) -> ! {
                runner.run().await
            }
            cyw43_task(runner)?
        });

        control.init(&FIRMWARE_CLM).await;
        control
            .set_power_management(cyw43::PowerManagementMode::None)
            .await;
        let mac_address = control.address().await;

        module = make_static!(
            RadioModule,
            RadioModule {
                control: Mutex::new(control),
                common: pio.common,
                irq_flags: pio.irq_flags,
                irq1: pio.irq1,
                irq2: pio.irq2,
                irq3: pio.irq3,
                sm1: pio.sm1,
                sm2: pio.sm2,
                sm3: pio.sm3,
            }
        );
        info!("{MODULE} Connected");
        Ok(RadioDrivers {
            module,
            #[cfg(feature = "ble")]
            ble: bt_device,
            #[cfg(feature = "wifi")]
            net: net_device,
            mac_address,
        })
    }
}

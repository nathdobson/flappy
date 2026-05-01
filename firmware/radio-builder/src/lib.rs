#![no_std]
#![feature(type_alias_impl_trait)]
#![deny(unused_must_use)]
#![feature(never_type)]
#![feature(allocator_api)]
extern crate alloc;

#[cfg(feature = "ble")]
pub mod ble;
mod error;
#[cfg(feature = "led")]
pub mod led;
#[cfg(feature = "wifi")]
pub mod wifi;
#[cfg(feature = "ble")]
pub mod ble_rpc;

pub use error::Error;

use cyw43::{A4, Aligned, Control, aligned_bytes};
use cyw43_pio::PioSpi;
use embassy_executor::{SpawnError, Spawner};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::interrupt::typelevel::{Binding, DMA_IRQ_0, PIO0_IRQ_0};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_rp::pio::{Common, Irq, IrqFlags, Pio, StateMachine};
use embassy_rp::{Peri, dma, pio};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use log::info;
use make_static::make_static;

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

pub struct RadioBuilder<I1, I2> {
    pub spawner: Spawner,
    pub peripherals: RadioPeripherals,
    pub pio_irq: I1,
    pub dma_irq: I2,
}

struct RadioModule {
    control: Mutex<NoopRawMutex, Control<'static>>,
    // Dropping this causes weird stuff to happen
    _common: Common<'static, PIO0>,
    _irq_flags: IrqFlags<'static, PIO0>,
    _irq1: Irq<'static, PIO0, 1>,
    _irq2: Irq<'static, PIO0, 2>,
    _irq3: Irq<'static, PIO0, 3>,
    _sm1: StateMachine<'static, PIO0, 1>,
    _sm2: StateMachine<'static, PIO0, 2>,
    _sm3: StateMachine<'static, PIO0, 3>,
}

pub struct Radio {
    #[cfg(feature = "led")]
    pub led: &'static crate::led::RadioLed,
    #[cfg(feature = "ble")]
    pub ble: crate::ble::BlePeripherals,
    #[cfg(feature = "wifi")]
    pub wifi: crate::wifi::WifiPeripherals,
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

impl<I1, I2> RadioBuilder<I1, I2>
where
    I1: 'static + Binding<PIO0_IRQ_0, pio::InterruptHandler<PIO0>>,
    I2: 'static + Binding<DMA_IRQ_0, dma::InterruptHandler<DMA_CH0>>,
{
    pub async fn build(self) -> Result<Radio, SpawnError> {
        let module: &'static mut RadioModule;
        info!("[Radio] Connecting to CYW43 radio transceiver over PIO-SPI");
        let pwr = Output::new(self.peripherals.PIN_23, Level::Low);
        let cs = Output::new(self.peripherals.PIN_25, Level::High);
        let mut pio = Pio::new(self.peripherals.PIO0, self.pio_irq);
        let spi = PioSpi::new(
            &mut pio.common,
            pio.sm0,
            // SPI communication won't work if the speed is too high, so we use a divider larger than `DEFAULT_CLOCK_DIVIDER`.
            // See: https://github.com/embassy-rs/embassy/issues/3960.
            // This value seems to be pretty good to limit BLE corruption.
            #[cfg(feature = "ble")]
            fixed::FixedU32::from_bits(0x0B00),
            #[cfg(not(feature = "ble"))]
            cyw43_pio::RM2_CLOCK_DIVIDER,
            pio.irq0,
            cs,
            self.peripherals.PIN_24,
            self.peripherals.PIN_29,
            dma::Channel::new(self.peripherals.DMA_CH0, self.dma_irq),
        );

        let state = make_static!(cyw43::State, cyw43::State::new());
        #[cfg(feature = "ble")]
        let (net_device, bt_device, mut control, runner) =
            cyw43::new_with_bluetooth(state, pwr, spi, FIRMWARE, FIRMWARE_BTFW, FIRMWARE_NVRAM)
                .await;
        #[cfg(not(feature = "ble"))]
        let (net_device, mut control, runner) =
            cyw43::new(state, pwr, spi, FIRMWARE, FIRMWARE_NVRAM).await;

        self.spawner.spawn({
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
                _common: pio.common,
                _irq_flags: pio.irq_flags,
                _irq1: pio.irq1,
                _irq2: pio.irq2,
                _irq3: pio.irq3,
                _sm1: pio.sm1,
                _sm2: pio.sm2,
                _sm3: pio.sm3,
            }
        );
        info!("Radio Connected");
        Ok(Radio {
            #[cfg(feature = "led")]
            led: make_static!(
                crate::led::RadioLed,
                crate::led::RadioLed {
                    control: &module.control
                }
            ),
            #[cfg(feature = "ble")]
            ble: crate::ble::BlePeripherals {
                ble: bt_device,
                mac_address,
            },
            #[cfg(feature = "wifi")]
            wifi: crate::wifi::WifiPeripherals {
                net: net_device,
                control: &module.control,
            },
        })
    }
}

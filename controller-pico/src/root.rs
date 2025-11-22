use crate::ble::{BleModule, BleModuleBuilder, BleTask};
use crate::error::Error;
use crate::flash::{FlashModule, FlashModuleBuilder, FlashTask};
use crate::led::{LedModule, LedModuleBuilder, LedTask};
use crate::radio::{RadioModule, RadioModuleBuilder, RadioTask};
use crate::usb::{UsbModule, UsbModuleBuilder};
use crate::wifi::{WifiModule, WifiModuleBuilder};
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_rp::clocks::RoscRng;
use log::info;
use static_cell::StaticCell;

pub struct RootModuleBuilder {}

pub struct RootModule {
    pub usb: &'static UsbModule,
    pub flash: &'static FlashModule,
    pub ble: &'static BleModule,
    pub led: &'static LedModule,
    pub wifi: &'static WifiModule,
}

pub struct RootTask {
    ble_task: BleTask,
    flash_task: FlashTask,
    led_task: LedTask,
    radio_task: RadioTask,
}

impl RootTask {
    pub fn spawn(self, spawner: Spawner) -> Result<(), Error> {
        self.ble_task.spawn(spawner)?;
        self.flash_task.spawn(spawner)?;
        self.led_task.spawn(spawner)?;
        self.radio_task.spawn(spawner)?;
        Ok(())
    }
}

impl RootModuleBuilder {
    pub async fn build(self, spawner: Spawner) -> Result<(RootTask, &'static RootModule), Error> {
        let p = embassy_rp::init(Default::default());

        let usb: &'static UsbModule = match (UsbModuleBuilder {
            spawner,
            usb: p.USB,
        }
        .build())
        {
            Ok(usb) => usb,
            Err(e) => {
                panic!();
            }
        };
        info!("Welcome to the Split Flap Display!");
        yield_now().await;

        let mut rng = RoscRng;

        let (flash_task, flash) = FlashModuleBuilder {
            flash: p.FLASH,
            dma_ch: p.DMA_CH1.into(),
        }
        .build()
        .await?;
        let (radio_task, bt_device, net_device, radio) = RadioModuleBuilder {
            spawner,
            pin23: p.PIN_23,
            pin24: p.PIN_24,
            pin25: p.PIN_25,
            pin29: p.PIN_29,
            pio0: p.PIO0,
            dma_ch0: p.DMA_CH0.into(),
        }
        .build()
        .await?;

        let (ble_task, ble) = BleModuleBuilder { spawner, bt_device }.build().await?;

        let (led_task, led) = LedModuleBuilder { spawner, radio }.build().await?;

        let wifi = WifiModuleBuilder {
            spawner,
            rng: &mut rng,
            net_device,
            radio,
        }
        .build()
        .await?;

        static MODULE: StaticCell<RootModule> = StaticCell::new();
        let module = MODULE.init(RootModule {
            usb,
            flash,
            ble,
            wifi,
            led,
        });
        Ok((
            RootTask {
                ble_task,
                flash_task,
                led_task,
                radio_task,
            },
            module,
        ))
    }
}

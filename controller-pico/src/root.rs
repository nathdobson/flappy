use crate::ble::{BleHandler, BleModule, BleModuleBuilder, BleTask};
use crate::driver::{DriverBuilder, DriverModule};
use crate::error::Error;
use crate::flash::{FlashModule, FlashModuleBuilder, FlashTask};
use crate::led::{LedModule, LedModuleBuilder, LedTask};
use crate::mqtt::{MqttHandler, MqttModule, MqttModuleBuilder, MqttTask};
use crate::radio::{RadioModule, RadioModuleBuilder, RadioTask};
use crate::usb::{UsbModule, UsbModuleBuilder};
use crate::wifi::{WifiHandler, WifiModule, WifiModuleBuilder, WifiTask};
use core::any::Any;
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_rp::clocks::RoscRng;
use embassy_time::Timer;
use log::info;
use static_cell::StaticCell;

pub struct RootModuleBuilder {
    pub spawner: Spawner,
}

pub struct RootModule {
    pub usb: &'static UsbModule,
    pub flash: &'static FlashModule,
    pub ble: &'static BleModule,
    pub led: &'static LedModule,
    pub wifi: &'static WifiModule,
    pub mqtt: &'static MqttModule,
    pub driver: &'static DriverModule,
}

pub struct RootTask {
    ble_task: BleTask,
    wifi_task: WifiTask,
    flash_task: FlashTask,
    led_task: LedTask,
    radio_task: RadioTask,
    mqtt_task: MqttTask,
}

pub trait RootHandler: BleHandler + WifiHandler + MqttHandler {}
impl<T: BleHandler + WifiHandler + MqttHandler> RootHandler for T {}

impl RootTask {
    pub fn spawn(self, spawner: Spawner, module: &'static dyn RootHandler) -> Result<(), Error> {
        self.ble_task.spawn(spawner, module)?;
        self.wifi_task.spawn(spawner, module)?;
        self.flash_task.spawn(spawner)?;
        self.led_task.spawn(spawner)?;
        self.radio_task.spawn(spawner)?;
        self.mqtt_task.spawn(spawner, module)?;
        Ok(())
    }
}

impl RootModuleBuilder {
    pub async fn build(self) -> Result<(RootTask, &'static RootModule), Error> {
        let p = embassy_rp::init(Default::default());

        let usb: &'static UsbModule = match (UsbModuleBuilder {
            spawner: self.spawner,
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
        Timer::after_millis(2000).await;

        let driver = DriverBuilder {
            cipo: p.PIN_0,
            copi: p.PIN_3,
            clock: p.PIN_2,
            spi: p.SPI0,
            latch: p.PIN_1,
            load: p.PIN_4,
            reset: p.PIN_5,
            enable: p.PIN_6,
        }
        .build()
        .await?;

        let mut rng = RoscRng;

        let (flash_task, flash) = FlashModuleBuilder {
            flash: p.FLASH,
            dma_ch: p.DMA_CH1.into(),
        }
        .build()
        .await?;
        let (radio_task, bt_device, net_device, radio) = RadioModuleBuilder {
            spawner: self.spawner,
            pin23: p.PIN_23,
            pin24: p.PIN_24,
            pin25: p.PIN_25,
            pin29: p.PIN_29,
            pio0: p.PIO0,
            dma_ch0: p.DMA_CH0.into(),
        }
        .build()
        .await?;

        let (ble_task, ble) = BleModuleBuilder {
            spawner: self.spawner,
            bt_device,
        }
        .build()
        .await?;

        let (led_task, led) = LedModuleBuilder {
            spawner: self.spawner,
            radio,
        }
        .build()
        .await?;

        let (wifi_task, wifi) = WifiModuleBuilder {
            spawner: self.spawner,
            rng: &mut rng,
            net_device,
            radio,
        }
        .build()
        .await?;

        let (mqtt_task, mqtt) = MqttModuleBuilder {
            spawner: self.spawner,
            stack: &wifi.stack,
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
            mqtt,
            driver,
        });
        Ok((
            RootTask {
                ble_task,
                wifi_task,
                flash_task,
                led_task,
                radio_task,
                mqtt_task,
            },
            module,
        ))
    }
}

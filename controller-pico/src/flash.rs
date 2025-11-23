use crate::error::Error;
use crate::mqtt::MqttSettings;
use crate::wifi::WifiSettings;
use core::cell::RefCell;
use cortex_m::prelude::_embedded_hal_blocking_spi_Write;
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_rp::dma::AnyChannel;
use embassy_rp::flash::{Async, Flash, ERASE_SIZE};
use embassy_rp::peripherals::{DMA_CH1, FLASH};
use embassy_rp::Peri;
use log::{error, info};
use serde::{Deserialize, Serialize};
use static_cell::StaticCell;
use trouble_host::prelude::HeaplessString;

const ADDR_OFFSET: u32 = 0x110000;
const FLASH_SIZE: usize = 2 * 1024 * 1024;

pub struct FlashModuleBuilder {
    pub flash: Peri<'static, FLASH>,
    pub dma_ch: Peri<'static, AnyChannel>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct FlashSettings {
    pub wifi: WifiSettings,
    pub mqtt: MqttSettings,
}

#[repr(C, align(8))]
struct FlashFile([u8; ERASE_SIZE]);

pub struct FlashModule {
    flash: RefCell<Flash<'static, FLASH, Async, FLASH_SIZE>>,
}

pub struct FlashTask {}

impl FlashTask {
    pub fn spawn(self, spawner: Spawner) -> Result<(), Error> {
        Ok(())
    }
}

impl FlashModuleBuilder {
    pub async fn build(self) -> Result<(FlashTask, &'static FlashModule), Error> {
        let mut flash =
            embassy_rp::flash::Flash::<_, Async, FLASH_SIZE>::new(self.flash, self.dma_ch);

        static MODULE: StaticCell<FlashModule> = StaticCell::new();
        Ok((
            FlashTask {},
            MODULE.init(FlashModule {
                flash: RefCell::new(flash),
            }),
        ))
    }
}

impl FlashModule {
    pub async fn load(&self) -> Result<FlashSettings, Error> {
        let mut buf = FlashFile([0; ERASE_SIZE]);
        self.flash
            .borrow_mut()
            .read(ADDR_OFFSET, &mut buf.0)
            .await?;
        // info!("{:?}", buf.0);
        yield_now().await;
        match serde_json_core::from_slice::<FlashSettings>(&buf.0) {
            Ok((state, _)) => Ok(state),
            Err(x) => {
                error!("Failed to deserialize state {:?}", x);
                Ok(FlashSettings::default())
            }
        }
    }
    pub fn save(&self, state: &FlashSettings) -> Result<(), Error> {
        let mut data = serde_json_core::to_vec::<_, ERASE_SIZE>(&state)?;
        while data.len() < data.capacity() {
            data.push(b' ').unwrap();
        }
        let data = FlashFile(data.into_array().unwrap());
        info!("Erasing");
        self.flash
            .borrow_mut()
            .blocking_erase(ADDR_OFFSET, ADDR_OFFSET + ERASE_SIZE as u32)?;
        // info!("writing {:?}", data.0);
        self.flash
            .borrow_mut()
            .blocking_write(ADDR_OFFSET, &data.0)?;
        Ok(())
    }
}

use crate::error::Error;
use core::cell::RefCell;
use cortex_m::prelude::_embedded_hal_blocking_spi_Write;
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_rp::Peri;
use embassy_rp::dma::AnyChannel;
use embassy_rp::flash::{Async, ERASE_SIZE, Flash};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, FLASH};
use log::{error, info};
use protocol::setup::{AppSettings, WriteSettingsError};
use serde::{Deserialize, Serialize};
use static_cell::make_static;

const MODULE: &'static str = "[FLASH]";
const ADDR_OFFSET: u32 = 0x3E0000;
const FLASH_SIZE: usize = 4 * 1024 * 1024;

#[allow(non_snake_case)]
pub struct FlashPeripherals {
    pub FLASH: Peri<'static, FLASH>,
    pub DMA_CH1: Peri<'static, DMA_CH1>,
}

#[repr(C, align(8))]
struct FlashFile([u8; ERASE_SIZE]);

pub struct FlashModule {
    flash: RefCell<Flash<'static, FLASH, Async, FLASH_SIZE>>,
}

impl FlashModule {
    pub async fn new(peri: FlashPeripherals) -> Result<&'static FlashModule, Error> {
        let mut flash = Flash::<_, Async, FLASH_SIZE>::new(peri.FLASH, peri.DMA_CH1);
        let module = make_static!(FlashModule {
            flash: RefCell::new(flash),
        });
        Ok(module)
    }
}

impl FlashModule {
    pub async fn load(&self) -> Result<AppSettings, Error> {
        let mut buf = FlashFile([0; ERASE_SIZE]);
        let mut tmp = [0; ERASE_SIZE];
        self.flash
            .borrow_mut()
            .read(ADDR_OFFSET, &mut buf.0)
            .await?;
        yield_now().await;
        match serde_json_core::from_slice_escaped::<AppSettings>(&buf.0, &mut tmp) {
            Ok((state, _)) => Ok(state),
            Err(x) => {
                error!("{MODULE} Failed to deserialize state {:?}", x);
                Ok(AppSettings::default())
            }
        }
    }
    pub fn save(&self, state: &AppSettings) -> Result<(), WriteSettingsError> {
        let mut data = serde_json_core::to_vec::<_, ERASE_SIZE>(&state)
            .map_err(|_| WriteSettingsError::SerdeError)?;
        while data.len() < data.capacity() {
            data.push(b' ').unwrap();
        }
        let data = FlashFile(data.into_array().unwrap());
        self.flash
            .borrow_mut()
            .blocking_erase(ADDR_OFFSET, ADDR_OFFSET + ERASE_SIZE as u32)
            .map_err(|_| WriteSettingsError::FlashError)?;
        self.flash
            .borrow_mut()
            .blocking_write(ADDR_OFFSET, &data.0)
            .map_err(|_| WriteSettingsError::FlashError)?;
        Ok(())
    }
}

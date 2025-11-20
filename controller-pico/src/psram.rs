use core::slice;
use embassy_rp::peripherals::{PIN_0, QMI_CS1};
use embassy_rp::Peri;
use embassy_time::Timer;
use log::error;

pub struct PsramModuleBuilder {
    pub qmi_cs1: Peri<'static, QMI_CS1>,
    pub pin_0: Peri<'static, PIN_0>,
}
pub struct PsramModule {
    data: &'static mut [u8],
}

impl PsramModuleBuilder {
    pub async fn build(self) -> PsramModule {
        let psram_config = embassy_rp::psram::Config::aps6404l();
        let psram = embassy_rp::psram::Psram::new(
            embassy_rp::qmi_cs1::QmiCs1::new(self.qmi_cs1, self.pin_0),
            psram_config,
        );

        let Ok(psram) = psram else {
            error!("PSRAM not found");
            loop {
                Timer::after_secs(1).await;
            }
        };

        let psram_slice = unsafe {
            let psram_ptr = psram.base_address();
            let slice: &'static mut [u8] =
                slice::from_raw_parts_mut(psram_ptr, psram.size() as usize);
            slice
        };

        PsramModule { data: psram_slice }
    }
}

impl PsramModule {
    pub fn data(&mut self) -> &mut [u8] {
        self.data
    }
}

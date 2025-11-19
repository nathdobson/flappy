use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_rp::{bind_interrupts, Peri};
use embassy_usb_logger::ReceiverHandler;
use crate::runtime::reboot;

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
});

pub fn spawn_usb(spawner: Spawner, usb: Peri<'static, USB>) {}

pub struct UsbModuleBuilder {
    pub spawner: Spawner,
    pub usb: Peri<'static, USB>,
}

pub struct UsbModule {}

impl UsbModuleBuilder {
    #[must_use]
    pub fn build(self) -> UsbModule {
        let driver = Driver::new(self.usb, UsbIrqs);
        self.spawner.spawn(logger_task(driver)).unwrap();
        UsbModule {}
    }
}

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver, UsbInputHandler);
}

struct UsbInputHandler;
impl ReceiverHandler for UsbInputHandler {
    async fn handle_data(&self, data: &[u8]) {
        if data == &[3u8] {
            reboot();
            return;
        }
        if let Ok(data) = str::from_utf8(data) {
            let data = data.trim();
            if data.eq_ignore_ascii_case("hello") {
                log::info!("World!");
            } else {
                log::info!("Recieved: {:?}", data);
            }
        }
    }

    fn new() -> Self {
        Self
    }
}

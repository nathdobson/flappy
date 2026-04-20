use crate::error::Error;
use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::peripherals::USB;
use make_static::make_static;
use protocol::{PRODUCT_MANUFACTURER, PRODUCT_NAME};
use usb_builder::usb_reset::UsbResetServer;
use usb_builder::{UsbBuilder, UsbServer, UsbStack};
use usb_builder::usb_terminal::UsbTerminalServer;

#[derive(UsbServer)]
pub struct FlappyUsbServer {
    usb_reset_server: UsbResetServer,
    usb_terminal: UsbTerminalServer,
}

impl FlappyUsbServer {
    pub fn new() -> Self {
        FlappyUsbServer {
            usb_terminal: UsbTerminalServer::new(),
            usb_reset_server: UsbResetServer::new(),
        }
    }
    pub fn init(&'static self) {
        self.usb_terminal.set_logger();
    }
    pub async fn start(
        &'static self,
        spawner: Spawner,
        peri: Peri<'static, USB>,
    ) -> Result<(), Error> {
        let stack = make_static!(UsbStack<FlappyUsbServer>, UsbStack::new());
        UsbBuilder {
            server: self,
            stack: stack,
            peri,
            spawner,
            manufacturer: Some(PRODUCT_MANUFACTURER),
            product: Some(PRODUCT_NAME),
        }
        .build()?;
        Ok(())
    }
}

#![no_std]
#![feature(never_type)]
#![feature(macro_derive)]
#![feature(type_alias_impl_trait)]
#![allow(dead_code)]
#![allow(unused_features)]
#![deny(unused_must_use)]
#![deny(non_snake_case)]
#![feature(try_blocks)]
#![allow(unused_variables)]
#![allow(unreachable_code)]
#![allow(unused_mut)]
#![feature(allocator_api)]
#![feature(unwrap_infallible)]
extern crate alloc;

pub mod error;
mod test;
pub mod usb_reset;
mod usb_server;
pub mod usb_terminal;
mod watch;
pub mod usb_rpc;
mod boxed_channel;

use embassy_executor::Spawner;
use embassy_executor::raw::TaskPool;
pub use usb_server::UsbServer;

use crate::error::Error;
use crate::usb_server::Buffer;
use board_info::serial_number;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_rp::{Peri, bind_interrupts};
use embassy_usb::{Builder, Config, UsbDevice};
use protocol_usb::{PI_SERIAL_PRODUCT_ID, PI_VENDOR_ID};

pub mod reexports {
    pub use embassy_executor;
    pub use embassy_usb_driver;
    pub use embassy_usb;
    pub use embassy_rp;
}

pub const MAX_PACKET_SIZE: u8 = 64;

pub struct UsbStack<S: UsbServer> {
    config_desc_buf: S::ConfigDescBuffer,
    bos_descriptor_buf: S::BosDescBuffer,
    msos_descriptor: S::MsosDescBuffer,
    control_buf: [u8; 64],
    runner: TaskPool<Runner, 1>,
}

type Runner = impl 'static + Future<Output = !>;

pub struct UsbBuilder<S: UsbServer> {
    pub server: &'static S,
    pub stack: &'static mut UsbStack<S>,
    pub peri: Peri<'static, USB>,
    pub spawner: Spawner,
    pub manufacturer: Option<&'static str>,
    pub product: Option<&'static str>,
}

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

impl<S: UsbServer> UsbStack<S> {
    pub fn new() -> Self {
        UsbStack {
            config_desc_buf: S::ConfigDescBuffer::zeroed(),
            bos_descriptor_buf: S::BosDescBuffer::zeroed(),
            msos_descriptor: S::MsosDescBuffer::zeroed(),
            control_buf: [0; _],
            runner: TaskPool::new(),
        }
    }
}

#[define_opaque(Runner)]
fn runner(mut device: UsbDevice<'static, Driver<'static, USB>>) -> impl FnOnce() -> Runner {
    move || async move {
        device.run().await;
    }
}

impl<S: UsbServer> UsbBuilder<S> {
    pub fn build(self) -> Result<(), Error> {
        let driver = Driver::new(self.peri, Irqs);

        let mut config = Config::new(PI_VENDOR_ID, PI_SERIAL_PRODUCT_ID);
        config.manufacturer = self.manufacturer;
        config.product = self.product;

        config.serial_number = serial_number();
        config.max_power = 100;
        config.max_packet_size_0 = MAX_PACKET_SIZE;

        config.device_class = 0xef;
        config.device_sub_class = 0x02;
        config.device_protocol = 0x01;
        config.composite_with_iads = true;

        let mut builder = Builder::new(
            driver,
            config,
            self.stack.config_desc_buf.as_mut(),
            self.stack.bos_descriptor_buf.as_mut(),
            self.stack.msos_descriptor.as_mut(),
            self.stack.control_buf.as_mut(),
        );
        self.server.build(self.spawner, &mut builder)?;
        let device = builder.build();
        self.spawner.spawn(self.stack.runner.spawn(runner(device))?);
        Ok(())
    }
}

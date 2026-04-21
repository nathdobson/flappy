use crate::UsbServer;
use crate::error::Error;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_usb::Builder;

struct S1;
struct S2;

impl UsbServer for S1 {
    type ConfigDescBuffer = [u8; 1];
    type BosDescBuffer = [u8; 1];
    type MsosDescBuffer = [u8; 1];
    fn build(
        &'static self,
        _spawner: Spawner,
        _builder: &mut Builder<'static, embassy_rp::usb::Driver<'static, USB>>,
    ) -> Result<(), Error> {
        todo!()
    }
}
impl UsbServer for S2 {
    type ConfigDescBuffer = [u8; 2];
    type BosDescBuffer = [u8; 2];
    type MsosDescBuffer = [u8; 2];

    fn build(
        &'static self,
        _spawner: Spawner,
        _builder: &mut Builder<'static, embassy_rp::usb::Driver<'static, USB>>,
    ) -> Result<(), Error> {
        todo!()
    }
}

#[derive(UsbServer)]
struct TestServers {
    s1: S1,
    s2: S2,
}

fn bar<T: UsbServer>() {}

fn foo() {
    bar::<TestServers>();
}

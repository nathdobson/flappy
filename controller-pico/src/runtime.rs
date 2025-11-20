use core::intrinsics::abort;
use embassy_rp::rom_data;
use embassy_time::{block_for, Duration};
use log::error;
use crate::usb::flush_logger;

pub fn reboot() {
    rom_data::reboot(0x0002, 500, 0, 0);
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info);
    flush_logger();
}

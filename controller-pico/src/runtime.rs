use core::intrinsics::abort;
use embassy_rp::rom_data;
use embassy_time::{block_for, Duration};
use log::error;

pub fn reboot() {
    rom_data::reboot(0x0002, 500, 0, 0);
}




#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info);
    block_for(Duration::from_millis(10000));
    reboot();
    abort();
}

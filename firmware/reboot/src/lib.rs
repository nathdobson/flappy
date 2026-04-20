#![no_std]
use embassy_rp::rom_data::reboot;

const REBOOT2_FLAG_REBOOT_TYPE_BOOTSEL: u32 = 0x2;
const REBOOT2_FLAG_NO_RETURN_ON_SUCCESS: u32 = 0x100;

pub fn reboot_bootsel_now() -> ! {
    reboot(
        REBOOT2_FLAG_REBOOT_TYPE_BOOTSEL | REBOOT2_FLAG_NO_RETURN_ON_SUCCESS,
        10,
        0,
        0,
    );
    unreachable!();
}

pub fn reboot_bootsel_after(delay_ms: u32) {
    reboot(
        REBOOT2_FLAG_REBOOT_TYPE_BOOTSEL,
        delay_ms,
        0,
        0,
    );
}
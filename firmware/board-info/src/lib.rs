#![no_std]
use embassy_sync::lazy_lock::LazyLock;
use heapless::format;
use heapless::String;
use embassy_rp::otp::get_chipid;
static SERIAL_NUMBER: LazyLock<Option<String<128>>> =
    LazyLock::new(|| Some(format!("{:016X}", get_chipid().ok()?).ok()?));

pub fn serial_number() -> Option<&'static str> {
    Some(SERIAL_NUMBER.get().as_ref()?)
}

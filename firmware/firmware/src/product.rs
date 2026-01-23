use embassy_rp::otp::get_chipid;
use embassy_sync::lazy_lock::LazyLock;
use heapless::{String, format};

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

static SERIAL_NUMBER: LazyLock<Option<String<128>>> =
    LazyLock::new(|| Some(format!("{:016X}", get_chipid().ok()?).ok()?));

pub fn serial_number() -> Option<&'static str> {
    Some(SERIAL_NUMBER.get().as_ref()?)
}

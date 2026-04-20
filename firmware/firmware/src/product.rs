use embassy_rp::otp::get_chipid;
use heapless::{String, format};

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

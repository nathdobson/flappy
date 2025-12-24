#![no_std]

pub mod ble;
pub mod display;
pub mod setup;
#[cfg(test)]
mod test;
pub mod usb;
pub mod error;
// mod bytes;

pub const PRODUCT_MANUFACTURER: &str = "Burnt Out Robotics";
pub const PRODUCT_NAME: &str = "Split Flap Display";
pub const PRODUCT_SHORT_NAME: &str = "Flap";

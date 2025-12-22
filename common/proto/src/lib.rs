#![no_std]

#[cfg(test)]
mod test;
pub mod display;
pub mod setup;
pub mod ble;

pub const PRODUCT_MANUFACTURER: &str = "Burnt Out Robotics";
pub const PRODUCT_NAME: &str = "Split Flap Display";

#![deny(unused_must_use)]
#![allow(unused_imports)]
#![allow(dead_code)]
pub mod error;

#[cfg(feature = "ble")]
pub mod ble;
#[cfg(all(feature = "ble", test))]
mod ble_test;
mod serde;
#[cfg(feature = "usb")]
pub mod usb;
#[cfg(all(feature = "usb", test))]
mod usb_test;
pub mod client;

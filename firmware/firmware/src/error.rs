use core::fmt;
use core::fmt::{Display, Formatter};
use core::num::ParseIntError;
use embassy_executor::SpawnError;
use embassy_time::TimeoutError;
use heapless::CapacityError;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("spawn error")]
    SpawnError(#[from] SpawnError),

    #[cfg(feature = "flash")]
    #[error("flash error")]
    FlashError(#[from] embassy_rp::flash::Error),

    #[cfg(feature = "display")]
    #[error("SPI error")]
    SpiError(#[from] embassy_rp::spi::Error),

    #[error("capacity error")]
    CapacityError,

    #[cfg(feature = "usb")]
    #[error("USB builder error")]
    UsbBuilderError(#[from] usb_builder::error::Error),

    #[error("timeout waiting for user to press bootsel")]
    BootselButtonTimeout,

    #[cfg(feature = "radio")]
    #[error("radio builder error")]
    RadioBuilderError(#[from] radio_builder::Error),

    #[cfg(feature = "display")]
    #[error("error counting display segments")]
    CountFailure,

    #[cfg(feature = "ntp")]
    #[error("ntp error")]
    NtpError(#[from] ntp_builder::NtpError),

    #[cfg(feature = "display")]
    #[error("home error")]
    HomeError,
}

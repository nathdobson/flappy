#![no_std]

use core::fmt::{Display, Formatter};
use heapless::String;

#[derive(Default, Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WifiSettings {
    pub ssid: String<32>,
    pub password: String<63>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WifiStatus {
    Unconfigured,
    Disconnected,
    Connected,
    Error(u32),
}

impl Default for WifiStatus {
    fn default() -> Self {
        WifiStatus::Unconfigured
    }
}

impl Display for WifiStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

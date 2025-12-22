use crate::display::MAX_GLYPHS;
use core::fmt::{Display, Formatter};
use heapless::{String, Vec};
use crate::error::MqttServiceError;

pub const MAX_SETUP_MESSAGE_SIZE: usize = 1024;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SetupRequest {
    DeviceInfo,
    ReadSettings,
    WriteSettings(AppSettings),
    TouchAppStatus,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SetupResponse {
    DeviceInfo(DeviceInfo),
    ReadSettings(AppSettings),
    WriteSettings(Result<(), WriteSettingsError>),
    TouchAppStatus,
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MqttSettings {
    pub hostname: String<128>,
    pub port: u16,
    pub username: String<128>,
    pub password: String<128>,
    pub topic: String<128>,
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WifiSettings {
    pub ssid: String<32>,
    pub password: String<63>,
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DisplaySettings {
    pub calibration: Vec<usize, MAX_GLYPHS>,
}
#[derive(Default, Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AppSettings {
    pub wifi: WifiSettings,
    pub mqtt: MqttSettings,
    pub display: DisplaySettings,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeviceInfo {
    pub serial: u64,
    pub git_version: String<64>,
    pub git_dirty: Option<bool>,
    pub git_head_ref: String<64>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WriteSettingsError {
    SerdeError,
    FlashError,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MqttServiceStatus {
    Unconfigured,
    Disconnected,
    Connected,
    WaitingForLink,
    WaitingForDhcp,
    DnsQuery,
    TcpConnect,
    TlsConnect,
    MqttConnect,
    MqttSubscribe,
    Error(MqttServiceError),
}

impl Default for MqttServiceStatus {
    fn default() -> Self {
        MqttServiceStatus::Unconfigured
    }
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

impl Display for MqttServiceStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            MqttServiceStatus::Disconnected => write!(f, "Disconnected"),
            MqttServiceStatus::Connected => write!(f, "Connected"),
            MqttServiceStatus::WaitingForLink => write!(f, "Waiting for link"),
            MqttServiceStatus::WaitingForDhcp => write!(f, "Waiting for DHCP"),
            MqttServiceStatus::DnsQuery => write!(f, "Resolving hostname"),
            MqttServiceStatus::TcpConnect => write!(f, "Establishing TCP connection"),
            MqttServiceStatus::TlsConnect => write!(f, "Establishing TLS connection"),
            MqttServiceStatus::MqttConnect => write!(f, "Establishing MQTT connection"),
            MqttServiceStatus::MqttSubscribe => write!(f, "Subscribing to topic"),
            MqttServiceStatus::Error(e) => write!(f, "{}", e),
            MqttServiceStatus::Unconfigured => write!(f, "Unconfigured"),
        }
    }
}

impl Display for MqttServiceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AppStatus {
    pub mqtt_status: MqttServiceStatus,
    pub wifi_status: WifiStatus,
}

impl Display for AppStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} {}", self.wifi_status, self.mqtt_status)
    }
}

impl Display for WifiStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Display for WriteSettingsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

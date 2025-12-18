use crate::error::Error;
use heapless::String;

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MqttSettings {
    pub hostname: String<128>,
    pub port: u16,
    pub username: String<128>,
    pub password: String<128>,
    pub topic: String<128>,
}

pub enum MqttStatus {
    Disconnected,
    Connected,
    WaitingForLink,
    WaitingForDhcp,
    DnsQuery,
    TcpConnect,
    TlsConnect,
    MqttConnect,
    MqttSubscribe,
    #[cfg(feature = "radio")]
    Error(Error),
}

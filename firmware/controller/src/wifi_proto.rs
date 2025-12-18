use heapless::String;

pub enum WifiStatus {
    Disconnected,
    LinkUp,
    DhcpUp,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WifiSettings {
    pub ssid: String<32>,
    pub password: String<63>,
}

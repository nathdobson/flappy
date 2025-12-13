use crate::display_proto::DisplaySettings;
use crate::mqtt_proto::MqttSettings;
use crate::wifi_proto::WifiSettings;

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FlashSettings {
    pub wifi: WifiSettings,
    pub mqtt: MqttSettings,
    pub display: DisplaySettings,
}

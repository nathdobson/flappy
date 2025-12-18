use crate::error::Error;
use core::intrinsics::unreachable;
use core::str::FromStr;
use embassy_time::Timer;
use log::{info, warn};
use trouble_host::prelude::*;

// GATT Server definition
#[gatt_server]
pub struct Server {
    pub flappy_service: FlappyService,
}

pub const FLAPPY_SERVICE_UUID: Uuid = uuid!("5af0b930-b9b5-11f0-b558-0800200c9a66");

/// Battery service
#[gatt_service(uuid = FLAPPY_SERVICE_UUID)]
pub struct FlappyService {
    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "WiFi SSID")]
    #[characteristic(uuid = "71fe3670-b9b5-11f0-b558-0800200c9a66", read, write)]
    pub wifi_ssid: HeaplessString<32>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "WiFi Password")]
    #[characteristic(uuid = "62c24b3b-fe0c-47c4-85cc-18eb853a8f43", write)]
    pub wifi_password: HeaplessString<63>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "WiFi Status")]
    #[characteristic(uuid = "b63a97a1-ee72-4f44-9f82-c04c95c7d76e", read, notify)]
    pub wifi_status: HeaplessString<63>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "MQTT Hostname")]
    #[characteristic(uuid = "e789ca87-4fe2-4967-94ed-68f9d6cec087", read, write)]
    pub mqtt_hostname: HeaplessString<128>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "MQTT Port")]
    #[characteristic(uuid = "2b02c87e-260f-47d2-8fe0-83c87979fd01", read, write)]
    pub mqtt_port: HeaplessString<10>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "MQTT Username")]
    #[characteristic(uuid = "7a9cfbb2-78fa-4222-be11-ab7d401d6f08", read, write)]
    pub mqtt_username: HeaplessString<128>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "MQTT Password")]
    #[characteristic(uuid = "66629b6d-0c7f-45a4-aada-9dc5aea7341c", read, write)]
    pub mqtt_password: HeaplessString<128>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "MQTT Topic")]
    #[characteristic(uuid = "a2e39581-34e8-4613-b993-a50a32820841", read, write)]
    pub mqtt_topic: HeaplessString<128>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "MQTT Status")]
    #[characteristic(uuid = "ffa3be06-af36-4896-b336-02124a4dc539", read, notify)]
    pub mqtt_status: HeaplessString<128>,
}

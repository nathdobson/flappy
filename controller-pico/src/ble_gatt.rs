use crate::ble::MyPeripheral;
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

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "IRC Hostname")]
    #[characteristic(uuid = "e789ca87-4fe2-4967-94ed-68f9d6cec087", read, write)]
    pub irc_hostname: HeaplessString<256>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "IRC Port")]
    #[characteristic(uuid = "2b02c87e-260f-47d2-8fe0-83c87979fd01", read, write)]
    pub irc_port: HeaplessString<10>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "IRC Nickname")]
    #[characteristic(uuid = "7a9cfbb2-78fa-4222-be11-ab7d401d6f08", read, write)]
    pub irc_nickname: HeaplessString<10>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "IRC Channel")]
    #[characteristic(uuid = "a2e39581-34e8-4613-b993-a50a32820841", read, write)]
    pub irc_channel: HeaplessString<100>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "IRC Status")]
    #[characteristic(uuid = "ffa3be06-af36-4896-b336-02124a4dc539", read, notify)]
    pub irc_status: HeaplessString<100>,
}

pub const HANDLE_LIMIT: usize = 70;

//
// /// Create an advertiser to use to connect to a BLE Central, and wait for it to connect.
//
//
// /// Example task to use the BLE notifier interface.
// /// This task will notify the connected central of a counter value every 2 seconds.
// /// It will also read the RSSI value every 2 seconds.
// /// and will stop when the connection is closed by the central or an error occurs.
// pub async fn custom_task<C: Controller, P: PacketPool>(
//     server: &Server<'_>,
//     conn: &GattConnection<'_, '_, P>,
//     stack: &Stack<'_, C, P>,
// ) {
//     let mut tick: u8 = 0;
//     let wifi_ssid = &server.flappy_service.wifi_ssid;
//     loop {
//         tick = tick.wrapping_add(1);
//         // info!("[custom_task] notifying connection of tick {}", tick);
//         // let mut formatted = HeaplessString::new();
//         // {
//         //     use core::fmt::Write;
//         //     write!(&mut formatted, "{}", tick).ok();
//         // }
//         // if wifi_ssid.notify(conn, &formatted).await.is_err() {
//         //     info!("[custom_task] error notifying connection");
//         //     break;
//         // };
//         // read RSSI (Received Signal Strength Indicator) of the connection.
//         if let Ok(rssi) = conn.raw().rssi(stack).await {
//             info!("[custom_task] RSSI: {:?}", rssi);
//         } else {
//             info!("[custom_task] error getting RSSI");
//             break;
//         };
//         Timer::after_secs(2).await;
//     }
// }

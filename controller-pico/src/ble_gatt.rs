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
    flappy_service: FlappyService,
}

const FLAPPY_SERVICE_UUID: Uuid = uuid!("5af0b930-b9b5-11f0-b558-0800200c9a66");
const FLAPPY_SERVICE_UUID_BYTES: [u8; 16] = {
    match FLAPPY_SERVICE_UUID {
        Uuid::Uuid16(_) => unreachable!(),
        Uuid::Uuid128(x) => x,
    }
};

/// Battery service
#[gatt_service(uuid = FLAPPY_SERVICE_UUID)]
pub struct FlappyService {
    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "WiFi SSID")]
    #[characteristic(uuid = "71fe3670-b9b5-11f0-b558-0800200c9a66", read, write)]
    wifi_ssid: HeaplessString<32>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "WiFi Password")]
    #[characteristic(uuid = "62c24b3b-fe0c-47c4-85cc-18eb853a8f43", write)]
    wifi_password: HeaplessString<63>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "WiFi Status")]
    #[characteristic(uuid = "b63a97a1-ee72-4f44-9f82-c04c95c7d76e", read, notify)]
    wifi_status: HeaplessString<63>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "IRC Hostname")]
    #[characteristic(uuid = "e789ca87-4fe2-4967-94ed-68f9d6cec087", read, write)]
    irc_hostname: HeaplessString<256>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "IRC Port")]
    #[characteristic(uuid = "2b02c87e-260f-47d2-8fe0-83c87979fd01", read, write)]
    irc_port: HeaplessString<10>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "IRC Nickname")]
    #[characteristic(uuid = "7a9cfbb2-78fa-4222-be11-ab7d401d6f08", read, write)]
    irc_nickname: HeaplessString<10>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "IRC Channel")]
    #[characteristic(uuid = "a2e39581-34e8-4613-b993-a50a32820841", read, write)]
    irc_channel: HeaplessString<100>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "IRC Status")]
    #[characteristic(uuid = "ffa3be06-af36-4896-b336-02124a4dc539", read, notify)]
    irc_status: HeaplessString<100>,
}

/// Stream Events until the connection closes.
///
/// This function will handle the GATT events and process them.
/// This is how we interact with read and write requests.
pub async fn gatt_events_task<P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
) -> Result<(), Error> {
    // let wifi_ssid = &server.flappy_service.wifi_ssid;
    // let wifi_password = &server.flappy_service.wifi_password;
    let reason = loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => break reason,
            GattConnectionEvent::Gatt { event } => {
                match &event {
                    GattEvent::Read(event) => {
                        if event.handle() == server.flappy_service.wifi_ssid.handle {
                            info!("[gatt] read wifi ssid");
                        } else if event.handle() == server.flappy_service.wifi_password.handle {
                            info!("[gatt] read wifi password");
                        } else if event.handle() == server.flappy_service.wifi_status.handle {
                            info!("[gatt] read wifi status");
                        } else if event.handle() == server.flappy_service.irc_hostname.handle {
                            info!("[gatt] read wifi irc hostname");
                        } else if event.handle() == server.flappy_service.irc_port.handle {
                            info!("[gatt] read wifi irc port");
                        } else if event.handle() == server.flappy_service.irc_nickname.handle {
                            info!("[gatt] read wifi irc nickname");
                        } else if event.handle() == server.flappy_service.irc_channel.handle {
                            info!("[gatt] read wifi irc channel");
                        } else if event.handle() == server.flappy_service.irc_status.handle {
                            info!("[gatt] read wifi irc status");
                        } else {
                            info!("[gatt] unknown read")
                        }
                    }
                    GattEvent::Write(event) => {
                        if event.handle() == server.flappy_service.wifi_ssid.handle {
                            info!("[gatt] write wifi ssid");
                        } else if event.handle() == server.flappy_service.wifi_password.handle {
                            info!("[gatt] write wifi password");
                        } else if event.handle() == server.flappy_service.wifi_status.handle {
                            info!("[gatt] write wifi status");
                        } else if event.handle() == server.flappy_service.irc_hostname.handle {
                            info!("[gatt] write wifi irc hostname");
                        } else if event.handle() == server.flappy_service.irc_port.handle {
                            info!("[gatt] write wifi irc port");
                        } else if event.handle() == server.flappy_service.irc_nickname.handle {
                            info!("[gatt] write wifi irc nickname");
                        } else if event.handle() == server.flappy_service.irc_channel.handle {
                            info!("[gatt] write wifi irc channel");
                        } else if event.handle() == server.flappy_service.irc_status.handle {
                            info!("[gatt] write wifi irc status");
                        } else {
                            info!("[gatt] unknown write")
                        }
                    }
                    _ => {}
                };
                // This step is also performed at drop(), but writing it explicitly is necessary
                // in order to ensure reply is sent.
                match event.accept() {
                    Ok(reply) => reply.send().await,
                    Err(e) => warn!("[gatt] error sending response: {:?}", e),
                };
            }
            _ => {} // ignore other Gatt Connection Events
        }
    };
    info!("[gatt] disconnected: {:?}", reason);
    Ok(())
}

/// Create an advertiser to use to connect to a BLE Central, and wait for it to connect.
pub async fn advertise<'values, 'server>(
    name: &'values str,
    peripheral: &mut MyPeripheral<'values>,
    server: &'server Server<'values>,
) -> Result<GattConnection<'values, 'server, DefaultPacketPool>, Error> {
    let mut advertiser_data = [0; 128];
    let mut service_uuid = FLAPPY_SERVICE_UUID_BYTES;
    service_uuid.reverse();
    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids16(&[[0x0f, 0x18]]),
            AdStructure::ServiceUuids128(&[service_uuid]),
            AdStructure::CompleteLocalName(name.as_bytes()),
        ],
        &mut advertiser_data[..],
    )?;
    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &advertiser_data[..len],
                scan_data: &[],
            },
        )
        .await?;
    info!("[adv] advertising");
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[adv] connection established");
    Ok(conn)
}

/// Example task to use the BLE notifier interface.
/// This task will notify the connected central of a counter value every 2 seconds.
/// It will also read the RSSI value every 2 seconds.
/// and will stop when the connection is closed by the central or an error occurs.
pub async fn custom_task<C: Controller, P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
    stack: &Stack<'_, C, P>,
) {
    let mut tick: u8 = 0;
    let wifi_ssid = &server.flappy_service.wifi_ssid;
    loop {
        tick = tick.wrapping_add(1);
        // info!("[custom_task] notifying connection of tick {}", tick);
        // let mut formatted = HeaplessString::new();
        // {
        //     use core::fmt::Write;
        //     write!(&mut formatted, "{}", tick).ok();
        // }
        // if wifi_ssid.notify(conn, &formatted).await.is_err() {
        //     info!("[custom_task] error notifying connection");
        //     break;
        // };
        // read RSSI (Received Signal Strength Indicator) of the connection.
        if let Ok(rssi) = conn.raw().rssi(stack).await {
            info!("[custom_task] RSSI: {:?}", rssi);
        } else {
            info!("[custom_task] error getting RSSI");
            break;
        };
        Timer::after_secs(2).await;
    }
}

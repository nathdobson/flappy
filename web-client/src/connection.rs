use crate::error::Error;
use crate::status::{Status, StatusPriority};
use crate::utils::try_window;
use futures_util::StreamExt;
use nusb::device_info_from_wasm;
use picoboot::Picoboot;
use protocol::usb::VENDOR_ID;
use setup_client::ble::BleClientBuilder;
use setup_client::client::{Client, ClientTransport};
use setup_client::usb::{BootSelect, UsbClientBuilder};
use std::rc::Rc;
use web_sys::{Usb, UsbDeviceFilter, UsbDeviceRequestOptions};

pub enum EitherClient {
    Application(Client),
    Picoboot(Picoboot),
}
pub async fn connect_ble(status: Rc<Status>) -> Result<EitherClient, Error> {
    status.set(StatusPriority::Info, "Starting BLE scan...".to_string());
    let mut stream = BleClientBuilder::scan().await?;
    status.set(StatusPriority::Info, "Scanning...".to_string());
    let client = stream.next().await.ok_or(Error::BluetoothNotSupported)??;
    status.set(StatusPriority::Info, "Connecting to device...".to_string());
    let client = client.connect().await?;
    status.set(
        StatusPriority::Info,
        "Press white button on microcontroller.".to_string(),
    );
    let client = Client::BleClient(client);
    client.ping().await?;
    status.set(StatusPriority::Info, "Bluetooth: connected!".to_string());
    Ok(EitherClient::Application(client))
}

pub async fn connect_usb(status: Rc<Status>) -> Result<EitherClient, Error> {
    let usb: Usb = try_window()?
        .navigator()
        .usb()
        .ok_or(Error::UsbNotSupported)?;
    let filter = UsbDeviceFilter::new();
    filter.set_vendor_id(VENDOR_ID);
    let device = usb
        .request_device(&UsbDeviceRequestOptions::new(&[filter]))
        .await?;
    status.set(
        StatusPriority::Info,
        "USB: Opening connection...".to_string(),
    );
    let device = device_info_from_wasm(device).await?;
    let client = UsbClientBuilder::from_device_info(device);
    match client.boot_select() {
        BootSelect::Application => {
            let client = EitherClient::Application(Client::UsbClient(client.connect().await?));
            status.set(StatusPriority::Info, "USB: connected!".to_string());
            Ok(client)
        }
        BootSelect::Picoboot => {
            let client = EitherClient::Picoboot(client.connect_picoboot().await?);
            status.set(
                StatusPriority::Info,
                "USB: connected (picoboot)!".to_string(),
            );
            Ok(client)
        }
    }
}

pub async fn connect(
    transport: ClientTransport,
    status: Rc<Status>,
) -> Result<EitherClient, Error> {
    match transport {
        ClientTransport::Usb => Ok(connect_usb(status).await?),
        ClientTransport::Ble => Ok(connect_ble(status).await?),
    }
}

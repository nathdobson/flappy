use crate::error::Error;
use crate::status::{Status, StatusPriority};
use crate::utils::try_window;
use futures_util::StreamExt;
use log::info;
use nusb::device_info_from_wasm;
use protocol::setup::{
    AppSettings, AppStatus, DeviceInfo, SetupRequest, SetupResponse, WriteAppSettings,
};
use protocol::usb::VENDOR_ID;
use setup_client_lib::ble::{BleClient, BleClientBuilder};
use setup_client_lib::client::{Client, ClientTransport};
use setup_client_lib::usb::{UsbClient, UsbClientBuilder};
use std::rc::Rc;
use web_sys::{Usb, UsbDeviceFilter, UsbDeviceRequestOptions};

pub async fn connect_ble(status: Rc<Status>) -> Result<Client, Error> {
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
    status.set(StatusPriority::Info, "Connected!".to_string());
    Ok(client)
}

pub async fn connect_usb(status: Rc<Status>) -> Result<Client, Error> {
    let usb: Usb = try_window()?.navigator().usb();
    let mut filter = UsbDeviceFilter::new();
    filter.set_vendor_id(VENDOR_ID);
    let device = usb
        .request_device(&UsbDeviceRequestOptions::new(&[filter]))
        .await?;
    status.set(
        StatusPriority::Info,
        "USB: Opening connection...".to_string(),
    );
    Ok(Client::UsbClient(
        UsbClientBuilder::from_device_info(device_info_from_wasm(device).await?)
            .connect()
            .await?,
    ))
}

pub async fn connect(transport: ClientTransport, status: Rc<Status>) -> Result<Client, Error> {
    match transport {
        ClientTransport::Usb => Ok(connect_usb(status).await?),
        ClientTransport::Ble => Ok(connect_ble(status).await?),
    }
}

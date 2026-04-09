use crate::error::Error;
use crate::event_listener::{EventListener, EventType};
use crate::status::{Status, StatusPriority};
use crate::tabs::TabContent;
use crate::utils::{bluetooth, create_element};
use crate::utils::{try_window, AppendChild};
use log::info;
use protocol::ble::{APP_STATUS_UUID, FLAPPY_SERVICE_UUID};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
    Blob, BlobPropertyBag, Bluetooth, BluetoothLeScanFilterInit, Event, File, FileSystemFileHandle,
    HtmlDivElement, HtmlElement, Request, RequestDeviceOptions, Response, Text, Url,
};

use crate::ble_connection::BleConnection;
use itertools::Itertools;
use js_sys::{Array, ArrayBuffer, JsString, Uint8Array};
use jsonformat::Indentation;
use protocol::setup::{AppSettings, AppStatus, MAX_SETUP_MESSAGE_SIZE};
use std::future::IntoFuture;
use std::iter::Once;
use wasm_bindgen::{JsCast, JsValue};

pub struct SetupTab {
    node: HtmlDivElement,
    connect_listener: OnceCell<EventListener<'static>>,
    read_listener: OnceCell<EventListener<'static>>,
    write_listener: OnceCell<EventListener<'static>>,
    connection: RefCell<Option<Rc<BleConnection>>>,
    connect_status: Rc<Status>,
    device_info: HtmlDivElement,
    wifi_status: HtmlDivElement,
    mqtt_status: HtmlDivElement,
}

impl TabContent for SetupTab {
    fn title(&self) -> &str {
        "Configure Display"
    }

    fn id(&self) -> &str {
        "setup"
    }

    fn node(&self) -> &HtmlElement {
        &self.node
    }
}

impl SetupTab {
    pub fn new() -> Result<Rc<Self>, Error> {
        let node = create_element::<"div">()?;
        node.set_class_name("setup-tab");

        let connect_section = node.append_element::<"div">()?;
        connect_section.set_class_name("setup-section");
        let connect_status = Status::new()?;
        connect_section.append_child(connect_status.node())?;
        connect_status.set(StatusPriority::Info, "Display not connected".to_string());

        let device_section = node.append_element::<"div">()?;
        device_section.set_class_name("setup-section");
        let device_info = device_section.append_element::<"div">()?;
        device_info.set_text_content(Some("Device Info: n/a"));
        let wifi_status = device_section.append_element::<"div">()?;
        wifi_status.set_text_content(Some("Wifi: n/a"));
        let mqtt_status = device_section.append_element::<"div">()?;
        mqtt_status.set_text_content(Some("MQTT: n/a"));

        let connect_button = connect_section.append_element::<"button">()?;
        connect_button.append_text("Connect via Bluetooth")?;
        let read_button = node.append_element::<"button">()?;
        read_button.append_text("Read settings off device")?;
        let write_button = node.append_element::<"button">()?;
        write_button.append_text("Write settings to device")?;

        let this = Rc::new(SetupTab {
            node,
            connect_listener: OnceCell::new(),
            read_listener: OnceCell::new(),
            write_listener: OnceCell::new(),
            connection: RefCell::new(None),
            connect_status,
            device_info,
            wifi_status,
            mqtt_status,
        });
        this.connect_listener
            .set(EventListener::new(
                connect_button.clone().into(),
                EventType::Click,
                this.weak_callback(Self::connect_ble),
            )?)
            .ok()
            .unwrap();
        this.read_listener
            .set(EventListener::new(
                read_button.clone().into(),
                EventType::Click,
                this.weak_callback(Self::read_settings),
            )?)
            .ok()
            .unwrap();
        this.write_listener
            .set(EventListener::new(
                write_button.clone().into(),
                EventType::Click,
                this.weak_callback(Self::write_settings),
            )?)
            .ok()
            .unwrap();
        Ok(this)
    }
    fn weak_callback(self: &Rc<Self>, callback: fn(Rc<Self>, Event)) -> impl Fn(Event) -> bool {
        let this = Rc::downgrade(self);
        move |event| {
            if let Some(this) = this.upgrade() {
                callback(this, event)
            }
            false
        }
    }
    fn connect_ble(self: Rc<Self>, event: Event) {
        spawn_local(async move {
            if let Err(e) = self.try_connect_ble().await {
                self.connect_status.set(
                    StatusPriority::Error,
                    format!("Failed to connect via bluetooth: {}", e),
                )
            }
        });
    }
    async fn try_connect_ble(&self) -> Result<(), Error> {
        self.connect_status.reset();
        self.connect_status
            .set(StatusPriority::Info, "Bluetooth: connecting...".to_string());
        let connection = BleConnection::new(
            self.connect_status.clone(),
            self.wifi_status.clone(),
            self.mqtt_status.clone(),
        )
        .await?;
        *self.connection.borrow_mut() = Some(connection);
        Ok(())
    }
    fn read_settings(self: Rc<Self>, event: Event) {
        spawn_local(async move {
            if let Err(e) = self.try_read_settings().await {
                self.connect_status.set(
                    StatusPriority::Error,
                    format!("Failed to read settings via bluetooth: {}", e),
                );
            }
        });
    }
    async fn try_read_settings(&self) -> Result<(), Error> {
        self.connect_status
            .set(StatusPriority::Info, "Reading settings...".to_string());
        let Some(connection) = self.connection.borrow().clone() else {
            return Err(Error::NotConnected);
        };
        let settings = connection.read_settings().await?;
        let settings = serde_json_core::to_string::<_, MAX_SETUP_MESSAGE_SIZE>(&settings)?;
        let settings = jsonformat::format(&settings, Indentation::FourSpace);
        let link = create_element::<"a">()?;
        let parts = js_sys::Array::new();
        parts.push(&Uint8Array::from(&*settings.as_bytes()));
        let props = BlobPropertyBag::new();
        props.set_type("text/plain");
        let file = Blob::new_with_u8_array_sequence_and_options(&parts, &props)?;
        let url = Url::create_object_url_with_blob(&file)?;
        link.set_href(&url);
        link.set_download("setup.json");
        link.click();
        Url::revoke_object_url(&link.href())?;
        self.connect_status.set(
            StatusPriority::Info,
            "Finished reading settings.".to_string(),
        );
        Ok(())
    }
    fn write_settings(self: Rc<Self>, event: Event) {
        spawn_local(async move {
            if let Err(e) = self.try_write_settings().await {
                self.connect_status.set(
                    StatusPriority::Error,
                    format!("Failed to write settings via bluetooth: {}", e),
                );
            }
        });
    }
    async fn try_write_settings(&self) -> Result<(), Error> {
        let Some(connection) = self.connection.borrow().clone() else {
            return Err(Error::NotConnected);
        };
        let x: Array<FileSystemFileHandle> =
            try_window()?.show_open_file_picker()?.into_future().await?;
        let file: FileSystemFileHandle = x
            .into_iter()
            .exactly_one()
            .ok()
            .ok_or(Error::ExpectedSingleFile)?;
        let file = file.get_file().await?.dyn_into::<File>()?;
        let file = file.text().await?.dyn_into::<JsString>()?;
        let file: String = file.into();
        let mut temp = vec![0; MAX_SETUP_MESSAGE_SIZE];
        let app_settings: AppSettings = serde_json_core::from_str_escaped(&file, &mut temp)?.0;
        self.connect_status
            .set(StatusPriority::Info, "Writing settings...".to_string());
        connection.write_settings(app_settings).await?;
        self.connect_status.set(
            StatusPriority::Info,
            "Finished writing settings.".to_string(),
        );
        Ok(())
    }
}

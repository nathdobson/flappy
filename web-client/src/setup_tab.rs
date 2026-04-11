use crate::error::Error;
use crate::event_listener::{EventListener, EventType};
use crate::status::{Status, StatusPriority};
use crate::tabs::TabContent;
use crate::utils::{bluetooth, create_element, sleep, spawn_local_joinable, JoinHandle};
use crate::utils::{try_window, AppendChild};
use log::{error, info};
use protocol::ble::{APP_STATUS_UUID, FLAPPY_SERVICE_UUID};
use std::cell::{Cell, OnceCell, Ref, RefCell};
use std::rc::Rc;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
    Blob, BlobPropertyBag, Bluetooth, BluetoothLeScanFilterInit, Event, File, FileSystemFileHandle,
    HtmlButtonElement, HtmlDivElement, HtmlElement, Request, RequestDeviceOptions, Response, Text,
    Url,
};

use crate::bind_weak::{bind_weak_async_fn1, bind_weak_try_async_fn1};
use crate::ble_connection::BleConnection;
use crate::connection::{Connection, ConnectionType};
use crate::field;
use crate::usb_connection::UsbConnection;
use crate::value_editor::struct_editor::{Field, StructEditor};
use crate::value_editor::text_editor::TextEditor;
use crate::value_editor::value_form::ValueForm;
use empty_rc::EmptyRc;
use itertools::Itertools;
use js_sys::{Array, ArrayBuffer, JsString, Uint8Array};
use jsonformat::Indentation;
use protocol::setup::{
    AppSettings, AppStatus, SetupRequest, WifiSettings, WriteAppSettings, MAX_SETUP_MESSAGE_SIZE,
};
use std::future::IntoFuture;
use std::iter::Once;
use wasm_bindgen::{JsCast, JsValue};

pub struct SetupTab {
    node: HtmlDivElement,
    connect_usb_listener: EventListener<'static>,
    connect_usb_button: HtmlButtonElement,
    connect_ble_listener: EventListener<'static>,
    connect_ble_button: HtmlButtonElement,
    disconnect_listener: EventListener<'static>,
    disconnect_button: HtmlButtonElement,
    read_listener: EventListener<'static>,
    write_listener: EventListener<'static>,
    connection: RefCell<Option<ConnectionTask>>,
    connect_status: Rc<Status>,

    serial_number: HtmlDivElement,
    firmware_version: HtmlDivElement,
    glyph_count: HtmlDivElement,
    wifi_status: HtmlDivElement,
    mqtt_status: HtmlDivElement,
    wifi_settings: Rc<ValueForm<WifiSettings>>,
}

struct ConnectionTask {
    connection: Rc<OnceCell<Connection>>,
    task: JoinHandle<()>,
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
        let this = EmptyRc::<Self>::new();

        let node = create_element::<"div">()?;
        node.set_class_name("setup-tab");

        let connect_section = node.append_element::<"div">()?;
        connect_section.set_class_name("setup-section");
        let connect_status = Status::new()?;
        connect_section.append_child(connect_status.node())?;
        connect_status.set(StatusPriority::Info, "Display not connected".to_string());
        let connect_ble_button = connect_section.append_element::<"button">()?;
        connect_ble_button.append_text("Connect via Bluetooth")?;
        let connect_usb_button = connect_section.append_element::<"button">()?;
        connect_usb_button.append_text("Connect via USB")?;
        let disconnect_button = connect_section.append_element::<"button">()?;
        disconnect_button.append_text("Disconnect")?;

        let device_section = node.append_element::<"div">()?;
        device_section.set_class_name("setup-section device-info-section");
        device_section
            .append_element::<"div">()?
            .set_text_content(Some(&"Serial Number"));
        let serial_number = device_section.append_element::<"div">()?;
        device_section
            .append_element::<"div">()?
            .set_text_content(Some(&"Firmware Version"));
        let firmware_version = device_section.append_element::<"div">()?;
        device_section
            .append_element::<"div">()?
            .set_text_content(Some(&"Character Count"));
        let glyph_count = device_section.append_element::<"div">()?;
        device_section
            .append_element::<"div">()?
            .set_text_content(Some(&"Wifi Status"));
        let wifi_status = device_section.append_element::<"div">()?;
        device_section
            .append_element::<"div">()?
            .set_text_content(Some(&"MQTT Status"));
        let mqtt_status = device_section.append_element::<"div">()?;

        let wifi_section = node.append_element::<"div">()?;
        wifi_section.set_class_name("setup-section");
        wifi_section
            .append_element::<"div">()?
            .set_text_content(Some("Wifi Settings"));
        let mut wifi_struct = StructEditor::<WifiSettings>::new()?;
        wifi_struct.add(field!(ssid), TextEditor::new()?)?;
        wifi_struct.add(field!(password), TextEditor::new()?)?;
        let wifi_settings = ValueForm::new(wifi_struct)?;
        wifi_settings.set_submit_name("Save Wifi Settings");
        wifi_settings.set_on_submit(bind_weak_try_async_fn1(
            this.downgrade(),
            async move |this, settings| {
                this.write_settings_partial(WriteAppSettings {
                    wifi: Some(settings),
                    ..WriteAppSettings::default()
                })
                .await?;
                Ok(())
            },
        ));
        wifi_section.append_child(wifi_settings.node())?;

        let read_button = node.append_element::<"button">()?;
        read_button.append_text("Read settings file from display")?;
        let write_button = node.append_element::<"button">()?;
        write_button.append_text("Write settings file to display")?;

        let connect_ble_listener = EventListener::new(
            &connect_ble_button,
            EventType::Click,
            Self::weak_callback(&this, Self::connect_ble),
        )?;
        let connect_usb_listener = EventListener::new(
            &connect_usb_button,
            EventType::Click,
            Self::weak_callback(&this, Self::connect_usb),
        )?;
        let disconnect_listener = EventListener::new(
            &disconnect_button,
            EventType::Click,
            Self::weak_callback(&this, Self::disconnect),
        )?;
        let read_listener = EventListener::new(
            &read_button,
            EventType::Click,
            Self::weak_callback(&this, Self::read_settings),
        )?;
        let write_listener = EventListener::new(
            &write_button,
            EventType::Click,
            Self::weak_callback(&this, Self::write_settings),
        )?;
        let this = this.into_rc(SetupTab {
            node,
            connect_usb_button,
            connect_usb_listener,
            connect_ble_button,
            connect_ble_listener,
            disconnect_listener,
            disconnect_button,
            read_listener,
            write_listener,
            connection: RefCell::new(None),
            connect_status,

            serial_number,
            firmware_version,
            glyph_count,
            wifi_status,
            mqtt_status,
            wifi_settings,
        });
        this.show_connection(false)?;
        Ok(this)
    }
    fn show_connection(&self, connection: bool) -> Result<(), Error> {
        self.connect_ble_button
            .style()
            .set_property("display", if connection { "none" } else { "block" })?;
        self.connect_usb_button
            .style()
            .set_property("display", if connection { "none" } else { "block" })?;
        self.disconnect_button
            .style()
            .set_property("display", if connection { "block" } else { "none" })?;
        if !connection {
            self.serial_number.set_text_content(None);
            self.firmware_version.set_text_content(None);
            self.glyph_count.set_text_content(None);
            self.wifi_status.set_text_content(None);
            self.mqtt_status.set_text_content(None);
            self.wifi_settings.set_value(&WifiSettings::default());
            self.connect_status.reset();
            self.connect_status.set(StatusPriority::Info, String::new());
        }
        Ok(())
    }
    fn weak_callback(
        this: &EmptyRc<Self>,
        callback: fn(Rc<Self>, Event),
    ) -> impl Fn(Event) -> bool {
        let this = this.downgrade();
        move |event| {
            if let Some(this) = this.upgrade() {
                callback(this, event)
            }
            false
        }
    }
    fn spawn_connection(self: Rc<Self>, typ: ConnectionType) -> ConnectionTask {
        self.show_connection(true).ok();
        let cell = Rc::new(OnceCell::new());
        ConnectionTask {
            task: spawn_local_joinable({
                let cell = cell.clone();
                async move {
                    match Connection::new(typ, self.connect_status.clone()).await {
                        Ok(connection) => {
                            cell.set(connection.clone()).ok().unwrap();
                            if let Err(e) = self.run_connection(connection).await {
                                self.connect_status.set(
                                    StatusPriority::Error,
                                    format!("Connection failed: {}", e),
                                );
                            }
                        }
                        Err(e) => self.connect_status.set(
                            StatusPriority::Error,
                            format!("Error establishing connection: {}", e),
                        ),
                    }
                }
            }),
            connection: cell,
        }
    }
    async fn run_connection(&self, connection: Connection) -> Result<(), Error> {
        let device_info = connection.device_info().await?;
        info!("Device info: {:#?}", device_info);
        self.serial_number
            .set_text_content(Some(&format!("{:016x}", device_info.serial)));
        let dirty = if device_info.git_dirty == Some(true) {
            " (modified)"
        } else {
            ""
        };
        self.firmware_version
            .set_text_content(Some(&format!("{}{}", device_info.git_version, dirty)));
        self.glyph_count
            .set_text_content(Some(&format!("{}", device_info.glyphs)));
        let settings = connection.read_settings().await?;
        self.wifi_settings.set_value(&settings.wifi);
        // There's probably a bug in chrome's WebUSB implementation that drops packets, so we
        // need to run touch_app_status twice.
        connection.touch_app_status().await?;
        sleep(100).await;
        connection.touch_app_status().await?;
        info!("Receiving status");
        loop {
            let status = connection.next_status().await?;
            self.wifi_status
                .set_text_content(Some(&format!("{}", status.wifi_status)));
            self.mqtt_status
                .set_text_content(Some(&format!("{}", status.mqtt_status)));
        }
        Ok(())
    }
    fn connect_ble(self: Rc<Self>, event: Event) {
        self.connect_status.reset();
        self.connect_status
            .set(StatusPriority::Info, "Bluetooth: connecting...".to_string());
        self.connection
            .replace(Some(self.clone().spawn_connection(ConnectionType::Ble)));
    }
    fn connect_usb(self: Rc<Self>, event: Event) {
        self.connect_status.reset();
        self.connect_status
            .set(StatusPriority::Info, "USB: connecting...".to_string());
        self.connection
            .replace(Some(self.clone().spawn_connection(ConnectionType::Usb)));
    }
    fn disconnect(self: Rc<Self>, event: Event) {
        self.show_connection(false).ok();
        self.connection.replace(None);
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
    fn connection(&self) -> Result<Connection, Error> {
        Ok(self
            .connection
            .borrow()
            .as_ref()
            .ok_or(Error::NotConnected)?
            .connection
            .get()
            .ok_or(Error::NotConnected)?
            .clone())
    }
    async fn try_read_settings(&self) -> Result<(), Error> {
        self.connect_status
            .set(StatusPriority::Info, "Reading settings...".to_string());
        let connection = self.connection()?;
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
        let connection = self.connection()?;
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
        connection
            .write_settings(WriteAppSettings {
                wifi: Some(app_settings.wifi),
                mqtt: Some(app_settings.mqtt),
                display: Some(app_settings.display),
            })
            .await?;
        self.connect_status.set(
            StatusPriority::Info,
            "Finished writing settings.".to_string(),
        );
        Ok(())
    }
    async fn write_settings_partial(&self, settings: WriteAppSettings) -> Result<(), Error> {
        self.connection()?.write_settings(settings).await?;
        Ok(())
    }
}

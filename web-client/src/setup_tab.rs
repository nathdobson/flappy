use crate::bind_weak::{bind_weak_async_fn1, bind_weak_try_async_fn1};
use crate::connection::{EitherClient, connect, connect_usb};
use crate::error::Error;
use crate::event_listener::{EventListener, EventType};
use crate::field;
use crate::status::{Status, StatusPriority};
use crate::tabs::TabContent;
use crate::utils::{AppendChild, try_window};
use crate::utils::{JoinHandle, bluetooth, create_element, sleep, spawn_local_joinable};
use crate::value_editor::select_editor::SelectEditor;
use crate::value_editor::struct_editor::{Field, StructEditor};
use crate::value_editor::text_editor::TextEditor;
use crate::value_editor::value_form::ValueForm;
use btleplug::api::Central;
use btleplug::api::CentralEvent;
use btleplug::api::Manager;
use btleplug::api::Peripheral;
use btleplug::api::ScanFilter;
use empty_rc::EmptyRc;
use futures_util::StreamExt;
use itertools::Itertools;
use js_sys::{Array, ArrayBuffer, JsString, Uint8Array};
use jsonformat::Indentation;
use log::{error, info};
use picoboot::{Access, Picoboot};
use protocol_ble::uuid::{APP_STATUS_UUID, RPC_SERVICE_UUID};
use protocol_wifi::WifiSettings;

use protocol::setup::{
    AppSettings, AppStatus, DisplaySettings, DriverVersion, MAX_SETUP_MESSAGE_SIZE, MqttSettings,
    SetupRequest, WriteAppSettings,
};
use setup_client::ble::BleClient;
use setup_client::client::{Client, ClientTransport};
use setup_client::usb::UsbClient;
use std::cell::{Cell, OnceCell, Ref, RefCell};
use std::fmt::Display;
use std::future::IntoFuture;
use std::iter::Once;
use std::rc::Rc;
use std::str::FromStr;
use std::time::Duration;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    Blob, BlobPropertyBag, Bluetooth, BluetoothLeScanFilterInit, Event, File, FileSystemFileHandle,
    HtmlButtonElement, HtmlDivElement, HtmlElement, Request, RequestDeviceOptions, Response, Text,
    Url,
};

pub struct SetupTab {
    node: HtmlDivElement,
    connect_usb_listener: EventListener<'static>,
    connect_usb_button: HtmlButtonElement,
    connect_ble_listener: EventListener<'static>,
    connect_ble_button: HtmlButtonElement,
    disconnect_listener: EventListener<'static>,
    disconnect_button: HtmlButtonElement,
    connection: RefCell<Option<ConnectionTask>>,
    connect_status: Rc<Status>,

    serial_number: HtmlDivElement,
    firmware_version: HtmlDivElement,
    glyph_count: HtmlDivElement,
    wifi_status: HtmlDivElement,
    mqtt_status: HtmlDivElement,
    wifi_settings: Rc<ValueForm<WifiSettings>>,
    mqtt_settings: Rc<ValueForm<MqttSettings>>,
    display_settings: Rc<ValueForm<DisplaySettings>>,

    firmware_listener: EventListener<'static>,
    firmware_status: Rc<Status>,
}

struct ConnectionTask {
    connection: Rc<OnceCell<Rc<Client>>>,
    picoboot: Rc<Cell<Option<Picoboot>>>,
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

        let mqtt_section = node.append_element::<"div">()?;
        mqtt_section.set_class_name("setup-section");
        mqtt_section
            .append_element::<"div">()?
            .set_text_content(Some("MQTT Settings"));
        let mut mqtt_struct = StructEditor::<MqttSettings>::new()?;
        mqtt_struct.add(field!(hostname), TextEditor::new()?)?;
        mqtt_struct.add(field!(port), TextEditor::new()?)?;
        mqtt_struct.add(field!(username), TextEditor::new()?)?;
        mqtt_struct.add(field!(password), TextEditor::new()?)?;
        mqtt_struct.add(field!(topic), TextEditor::new()?)?;
        let mqtt_settings = ValueForm::new(mqtt_struct)?;
        mqtt_settings.set_submit_name("Save MQTT Settings");
        mqtt_settings.set_on_submit(bind_weak_try_async_fn1(
            this.downgrade(),
            async move |this, settings| {
                this.write_settings_partial(WriteAppSettings {
                    mqtt: Some(settings),
                    ..WriteAppSettings::default()
                })
                .await?;
                Ok(())
            },
        ));
        mqtt_section.append_child(mqtt_settings.node())?;

        let display_section = node.append_element::<"div">()?;
        display_section.set_class_name("setup-section");
        display_section
            .append_element::<"div">()?
            .set_text_content(Some("Display Settings"));
        let mut display_struct = StructEditor::<DisplaySettings>::new()?;
        display_struct.add(field!(calibration), TextEditor::new_json()?)?;
        display_struct.add(field!(glyphs), TextEditor::new_json()?)?;
        display_struct.add(field!(background), TextEditor::new()?)?;
        display_struct.add(field!(foreground), TextEditor::new()?)?;
        display_struct.add(
            field!(driver_version),
            SelectEditor::new(vec![DriverVersion::V1_0, DriverVersion::V2_0])?,
        )?;

        display_struct.add(field!(micros_per_tick), TextEditor::new_optional()?)?;
        display_struct.add(field!(slow_ticks_per_step), TextEditor::new_optional()?)?;
        display_struct.add(field!(slow_steps_per_stage), TextEditor::new_optional()?)?;
        display_struct.add(field!(fast_ticks_per_step), TextEditor::new_optional()?)?;
        display_struct.add(field!(rehome_after_stopping), TextEditor::new()?)?;
        let display_settings = ValueForm::new(display_struct)?;
        display_settings.set_submit_name("Save Display Settings");
        display_settings.set_on_submit(bind_weak_try_async_fn1(
            this.downgrade(),
            async move |this, settings| {
                this.write_settings_partial(WriteAppSettings {
                    display: Some(settings),
                    ..WriteAppSettings::default()
                })
                .await?;
                Ok(())
            },
        ));
        display_section.append_child(display_settings.node())?;

        let firmware_section = node.append_element::<"div">()?;
        firmware_section.set_class_name("setup-section");
        let firmware_button = firmware_section.append_element::<"button">()?;
        firmware_button.set_text_content(Some(&format!(
            "Flash firmware (version {})",
            crate::built_info::GIT_VERSION.unwrap_or("<unknown>")
        )));
        let firmware_listener = EventListener::new(
            &firmware_button,
            EventType::Click,
            Self::weak_callback(&this, Self::flash_firmware),
        )?;
        let firmware_status = Status::new()?;
        firmware_section.append_child(firmware_status.node())?;

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
        let this = this.into_rc(SetupTab {
            node,
            connect_usb_button,
            connect_usb_listener,
            connect_ble_button,
            connect_ble_listener,
            disconnect_listener,
            disconnect_button,
            connection: RefCell::new(None),
            connect_status,

            serial_number,
            firmware_version,
            glyph_count,
            wifi_status,
            mqtt_status,
            wifi_settings,
            mqtt_settings,
            display_settings,

            firmware_listener,
            firmware_status,
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
    ) -> impl 'static + Fn(Event) -> bool {
        let this = this.downgrade();
        move |event| {
            if let Some(this) = this.upgrade() {
                callback(this, event)
            }
            false
        }
    }
    fn spawn_connection(self: Rc<Self>, typ: ClientTransport) -> ConnectionTask {
        self.show_connection(true).ok();
        let client_cell = Rc::new(OnceCell::new());
        let picoboot_cell = Rc::new(Cell::new(None));
        ConnectionTask {
            task: spawn_local_joinable({
                let client_cell = client_cell.clone();
                let picoboot_cell = picoboot_cell.clone();
                async move {
                    match connect(typ, self.connect_status.clone()).await {
                        Ok(EitherClient::Application(client)) => {
                            let client = Rc::new(client);
                            client_cell.set(client.clone()).ok().unwrap();
                            if let Err(e) = self.run_connection(client).await {
                                self.connect_status.set(
                                    StatusPriority::Error,
                                    format!("Connection failed: {}", e),
                                );
                            }
                        }
                        Ok(EitherClient::Picoboot(client)) => {
                            picoboot_cell.set(Some(client));
                        }
                        Err(e) => self.connect_status.set(
                            StatusPriority::Error,
                            format!("Error establishing connection: {}", e),
                        ),
                    }
                }
            }),
            connection: client_cell,
            picoboot: picoboot_cell,
        }
    }
    async fn run_connection(&self, connection: Rc<Client>) -> Result<(), Error> {
        info!("Requesting device info...");
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
        self.mqtt_settings.set_value(&settings.mqtt);
        self.display_settings.set_value(&settings.display);
        // There's probably a bug in chrome's WebUSB implementation that drops packets, so we
        // need to run touch_app_status twice.
        connection.touch_app_status().await?;
        sleep(100).await;
        connection.touch_app_status().await?;
        info!("Receiving status");
        loop {
            let status = connection.receive_status().await?;
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
            .replace(Some(self.clone().spawn_connection(ClientTransport::Ble)));
    }
    fn connect_usb(self: Rc<Self>, event: Event) {
        self.connect_status.reset();
        self.connect_status
            .set(StatusPriority::Info, "USB: connecting...".to_string());
        self.connection
            .replace(Some(self.clone().spawn_connection(ClientTransport::Usb)));
    }
    fn disconnect(self: Rc<Self>, event: Event) {
        self.show_connection(false).ok();
        self.connection.replace(None);
    }
    fn connection(&self) -> Result<Rc<Client>, Error> {
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
    async fn write_settings_partial(&self, settings: WriteAppSettings) -> Result<(), Error> {
        self.connection()?.write_settings(settings).await?;
        Ok(())
    }
    fn flash_firmware(self: Rc<Self>, event: Event) {
        spawn_local(async move {
            self.firmware_status.reset();
            if let Err(e) = self.try_flash_firmware().await {
                self.firmware_status
                    .set(StatusPriority::Error, format!("{}", e));
            }
        })
    }
    async fn try_flash_firmware(&self) -> Result<(), Error> {
        self.firmware_status.set(
            StatusPriority::Info,
            "Retrieving firmware binary...".to_string(),
        );
        let binary: Response = try_window()?
            .fetch_with_str("./firmware.bin")
            .await?
            .dyn_into()?;
        let binary: ArrayBuffer = binary.array_buffer()?.into_future().await?.dyn_into()?;
        let binary = Uint8Array::new(&binary).to_vec();

        self.firmware_status.set(
            StatusPriority::Info,
            "Disconnecting from current device...".to_string(),
        );
        self.show_connection(false).ok();
        self.connection.replace(None);
        sleep(100).await;
        self.firmware_status
            .set(StatusPriority::Info, "Connecting...".to_string());

        let client = match connect_usb(self.firmware_status.clone()).await? {
            EitherClient::Application(Client::UsbClient(x)) => {
                self.firmware_status.set(
                    StatusPriority::Info,
                    "Resetting in picoboot mode...".to_string(),
                );
                if let Err(e) = x.reset_picoboot().await {
                    info!("error while resetting (this may be normal): {}", e);
                }
                self.firmware_status
                    .set(StatusPriority::Info, "Reconnecting...".to_string());
                sleep(100).await;
                match connect_usb(self.firmware_status.clone()).await? {
                    EitherClient::Application(_) => return Err(Error::NotPicobootMode),
                    EitherClient::Picoboot(client) => client,
                }
            }
            EitherClient::Picoboot(client) => client,
            EitherClient::Application(_) => unreachable!(),
        };
        self.firmware_status
            .set(StatusPriority::Info, "Connected to Picoboot...".to_string());

        let mut picoboot = Picoboot::from_first(None).await?;
        self.firmware_status.set(
            StatusPriority::Info,
            "Connecting to USB device...".to_string(),
        );
        let conn = picoboot.connect().await?;
        self.firmware_status
            .set(StatusPriority::Info, "Resetting interface...".to_string());
        conn.reset_interface().await?;
        self.firmware_status.set(
            StatusPriority::Info,
            "Disabling mass storage...".to_string(),
        );
        conn.set_exclusive_access(Access::ExclusiveAndEject).await?;
        self.firmware_status
            .set(StatusPriority::Info, "Disabling XIP...".to_string());
        conn.exit_xip().await?;
        self.firmware_status
            .set(StatusPriority::Info, "Erasing flash...".to_string());
        conn.flash_erase_start(binary.len()).await?;
        self.firmware_status
            .set(StatusPriority::Info, "Writing firmware...".to_string());
        conn.flash_write_start(&binary).await?;
        self.firmware_status
            .set(StatusPriority::Info, "Verifying firmware...".to_string());
        let verified = conn.flash_read_start(binary.len() as u32).await?;
        if binary != verified {
            self.firmware_status.set(
                StatusPriority::Error,
                format!(
                    "firmware verification failed comparing {} bytes and {} bytes",
                    binary.len(),
                    verified.len()
                ),
            );
            return Ok(());
        }
        self.firmware_status
            .set(StatusPriority::Info, "Rebooting device...".to_string());
        conn.reboot(Duration::from_millis(500)).await?;
        self.firmware_status.set(
            StatusPriority::Info,
            "Firmware successfully updated!".to_string(),
        );

        Ok(())
    }
}

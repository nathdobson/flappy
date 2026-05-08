use crate::bind_weak::bind_weak_try_async_fn1;
use crate::connection::{EitherClient, connect};
use crate::error::Error;
use crate::event_listener::{EventListenerSet, EventType};
use crate::field;
use crate::status::{Status, StatusPriority};
use crate::tabs::TabContent;
use crate::utils::AppendChild;
use crate::utils::{JoinHandle, create_element, sleep, spawn_local_joinable};
use crate::value_editor::input_editor::InputEditorBuilder;
use crate::value_editor::select_editor::SelectEditor;
use crate::value_editor::struct_editor::StructEditor;
use crate::value_editor::value_form::ValueForm;
use empty_rc::EmptyRc;
use log::info;
use picoboot::Picoboot;
use protocol_wifi::WifiSettings;

use crate::browser_support::{
    BROWSER_SUPPORT_MESSAGE, check_ble_supported, check_usb_supported, force_webview_to_chrome,
};
use crate::input_type::InputType;
use crate::value_editor::bool_editor::BoolEditor;
use crate::value_editor::calibration_editor::CalibrationEditor;
use crate::value_editor::list_editor::ListEditor;
use protocol::display::STEPS_PER_REVOLUTION;
use protocol::setup::{DisplaySettings, DriverVersion, MqttSettings, WriteAppSettings};
use setup_client::client::{Client, ClientTransport};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;
use web_sys::{Event, HtmlButtonElement, HtmlDivElement, HtmlElement};

pub struct SetupTab {
    node: HtmlDivElement,
    connect_usb_button: HtmlButtonElement,
    connect_ble_button: HtmlButtonElement,
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
    device_section: HtmlDivElement,
    wifi_section: HtmlDivElement,
    mqtt_section: HtmlDivElement,
    display_section: HtmlDivElement,

    #[allow(dead_code)]
    listeners: EventListenerSet<'static, Self>,
}

#[allow(dead_code)]
struct ConnectionTask {
    connection: Rc<OnceCell<Rc<Client>>>,
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
        let mut listeners = EventListenerSet::new(this.downgrade());

        let node = create_element::<"div">()?;
        node.set_class_name("setup-tab");

        let connect_section = node.append_element::<"div">()?;
        let ble_supported = check_ble_supported().is_ok();
        let usb_supported = check_usb_supported().is_ok();
        if !ble_supported || !usb_supported {
            force_webview_to_chrome()?;
            connect_section
                .append_element::<"p">()?
                .set_inner_html(BROWSER_SUPPORT_MESSAGE);
        }
        connect_section.set_class_name("setup-section");

        let connect_status = Status::new()?;
        connect_section.append_child(connect_status.node())?;
        connect_status.set(StatusPriority::Info, "Display not connected".to_string());

        let connect_ble_button = connect_section.append_element::<"button">()?;
        connect_ble_button.append_text("Connect via Bluetooth")?;
        connect_ble_button.set_disabled(!ble_supported);
        connect_ble_button.set_class_name("connect-button");
        listeners.add(&connect_ble_button, EventType::Click, Self::connect_ble)?;

        let connect_usb_button = connect_section.append_element::<"button">()?;
        connect_usb_button.append_text("Connect via USB")?;
        connect_usb_button.set_disabled(!usb_supported);
        connect_usb_button.set_class_name("connect-button");
        listeners.add(&connect_usb_button, EventType::Click, Self::connect_usb)?;

        let disconnect_button = connect_section.append_element::<"button">()?;
        disconnect_button.append_text("Disconnect")?;
        disconnect_button.set_class_name("connect-button");
        listeners.add(&disconnect_button, EventType::Click, Self::disconnect)?;

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
        let wifi_struct = StructEditor::<WifiSettings>::new()?;
        wifi_struct.add(
            field!("Wifi Name", ssid),
            InputEditorBuilder::new().with_from_str_display().build()?,
        )?;
        wifi_struct.add(
            field!("Wifi Password", password),
            InputEditorBuilder::new().with_from_str_display().build()?,
        )?;
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
        let mqtt_struct = StructEditor::<MqttSettings>::new()?;
        mqtt_struct.add(
            field!("Hostname", hostname),
            InputEditorBuilder::new().with_from_str_display().build()?,
        )?;
        mqtt_struct.add(
            field!("Port", port),
            InputEditorBuilder::new()
                .with_from_str_display()
                .with_type(InputType::Number)
                .with_min(0)
                .with_max(u16::MAX)
                .build()?,
        )?;
        mqtt_struct.add(
            field!("Username", username),
            InputEditorBuilder::new().with_from_str_display().build()?,
        )?;
        mqtt_struct.add(
            field!("Password", password),
            InputEditorBuilder::new().with_from_str_display().build()?,
        )?;
        mqtt_struct.add(
            field!("Topic", topic),
            InputEditorBuilder::new().with_from_str_display().build()?,
        )?;
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
        let display_struct = StructEditor::<DisplaySettings>::new()?;
        display_struct.add(
            field!("Calibrations", calibration),
            ListEditor::new(|| Ok(CalibrationEditor::new(STEPS_PER_REVOLUTION)?))?,
        )?;
        display_struct.add(
            field!("Glyphs", glyphs),
            InputEditorBuilder::new().with_json_serde().build()?,
        )?;
        display_struct.add(
            field!("Background", background),
            InputEditorBuilder::new_color().build()?,
        )?;
        display_struct.add(
            field!("Foreground", foreground),
            InputEditorBuilder::new_color().build()?,
        )?;
        display_struct.add(
            field!("Driver version", driver_version),
            SelectEditor::new(vec![DriverVersion::V1_0, DriverVersion::V2_0])?,
        )?;

        display_struct.add(
            field!("Microseconds per tick", micros_per_tick),
            InputEditorBuilder::new()
                .with_optional()
                .with_type(InputType::Number)
                .with_min(Some(1))
                .build()?,
        )?;
        display_struct.add(
            field!("Steps per stage (slow)", slow_steps_per_stage),
            InputEditorBuilder::new()
                .with_optional()
                .with_type(InputType::Number)
                .with_min(Some(1))
                .with_max(Some(u16::MAX))
                .build()?,
        )?;
        display_struct.add(
            field!("Ticks per step (slow)", slow_ticks_per_step),
            InputEditorBuilder::new()
                .with_optional()
                .with_type(InputType::Number)
                .with_min(Some(1))
                .with_max(Some(u8::MAX))
                .build()?,
        )?;
        display_struct.add(
            field!("Ticks per step (fast)", fast_ticks_per_step),
            InputEditorBuilder::new()
                .with_optional()
                .with_type(InputType::Number)
                .with_min(Some(1))
                .with_max(Some(u8::MAX))
                .build()?,
        )?;
        display_struct.add(
            field!("Re-home after stopping", rehome_after_stopping),
            BoolEditor::new()?,
        )?;
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

        let this = this.into_rc(SetupTab {
            node,
            connect_usb_button,
            connect_ble_button,
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
            listeners,

            device_section,
            wifi_section,
            mqtt_section,
            display_section,
        });
        this.show_connection(false)?;
        Ok(this)
    }
    fn show_connection(&self, connection: bool) -> Result<(), Error> {
        self.device_section
            .style()
            .set_property("display", "none")?;
        self.wifi_section.style().set_property("display", "none")?;
        self.mqtt_section.style().set_property("display", "none")?;
        self.display_section
            .style()
            .set_property("display", "none")?;
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
            self.wifi_settings.set_value(&WifiSettings::default())?;
            self.connect_status.reset();
            self.connect_status.set(StatusPriority::Info, String::new());
        }
        Ok(())
    }
    fn spawn_connection(self: Rc<Self>, typ: ClientTransport) -> ConnectionTask {
        self.show_connection(true).ok();
        let client_cell = Rc::new(OnceCell::new());
        ConnectionTask {
            task: spawn_local_joinable({
                let client_cell = client_cell.clone();
                async move {
                    match connect(typ, self.connect_status.clone()).await {
                        Ok(EitherClient::Application(client)) => {
                            let client = Rc::new(client);
                            client_cell.set(client.clone()).ok().unwrap();
                            let Err(e) = self.run_connection(client).await;
                            self.connect_status.set_error(
                                StatusPriority::Error,
                                "Connection failure",
                                &e,
                            );
                        }
                        Ok(EitherClient::Picoboot(client)) => {
                            self.connect_status
                                .set(StatusPriority::Error, "Device is in Boot Select Mode. Restart device in order to configure display.".to_string());
                        }
                        Err(e) => self.connect_status.set_error(
                            StatusPriority::Error,
                            "Connection failure",
                            &e,
                        ),
                    }
                }
            }),
            connection: client_cell,
        }
    }
    async fn run_connection(&self, connection: Rc<Client>) -> Result<!, Error> {
        self.device_section.style().remove_property("display")?;
        self.wifi_section.style().remove_property("display")?;
        self.mqtt_section.style().remove_property("display")?;
        self.display_section.style().remove_property("display")?;
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
        self.wifi_settings.set_value(&settings.wifi)?;
        self.mqtt_settings.set_value(&settings.mqtt)?;
        self.display_settings.set_value(&settings.display)?;
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
    }
    fn connect_ble(self: Rc<Self>, _event: Event) {
        self.connect_status.reset();
        self.connect_status
            .set(StatusPriority::Info, "Bluetooth: connecting...".to_string());
        self.connection
            .replace(Some(self.clone().spawn_connection(ClientTransport::Ble)));
    }
    fn connect_usb(self: Rc<Self>, _event: Event) {
        self.connect_status.reset();
        self.connect_status
            .set(StatusPriority::Info, "USB: connecting...".to_string());
        self.connection
            .replace(Some(self.clone().spawn_connection(ClientTransport::Usb)));
    }
    fn disconnect(self: Rc<Self>, _event: Event) {
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
}

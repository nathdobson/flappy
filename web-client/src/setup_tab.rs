use crate::error::Error;
use crate::event_listener::{EventListener, EventType};
use crate::status::{Status, StatusPriority};
use crate::tabs::TabContent;
use crate::utils::AppendChild;
use crate::utils::{bluetooth, create_element};
use log::info;
use protocol::ble::{APP_STATUS_UUID, FLAPPY_SERVICE_UUID};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
    Blob, BlobPropertyBag, Bluetooth, BluetoothLeScanFilterInit, HtmlDivElement, HtmlElement,
    RequestDeviceOptions, Url,
};

use crate::ble_connection::BleConnection;
use js_sys::{ArrayBuffer, Uint8Array};
use protocol::setup::{AppStatus, MAX_SETUP_MESSAGE_SIZE};
use std::future::IntoFuture;
use std::iter::Once;
use jsonformat::Indentation;

pub struct SetupTab {
    node: HtmlDivElement,
    connect_listener: OnceCell<EventListener<'static>>,
    read_listener: OnceCell<EventListener<'static>>,
    write_listener: OnceCell<EventListener<'static>>,
    connection: RefCell<Option<Rc<BleConnection>>>,
    connect_status: Rc<Status>,
    wifi_status: Rc<Status>,
    mqtt_status: Rc<Status>,
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
        let connect_button = node.append_element::<"button">()?;
        connect_button.append_text("Connect via Bluetooth")?;
        let read_button = node.append_element::<"button">()?;
        read_button.append_text("Read settings off device")?;
        let write_button = node.append_element::<"button">()?;
        write_button.append_text("Write settings to device")?;

        let connect_status = Status::new()?;
        node.append_child(connect_status.node())?;
        let wifi_status = Status::new()?;
        node.append_child(wifi_status.node())?;
        let mqtt_status = Status::new()?;
        node.append_child(mqtt_status.node())?;
        let setup_tab = Rc::new(SetupTab {
            node,
            connect_listener: OnceCell::new(),
            read_listener: OnceCell::new(),
            write_listener: OnceCell::new(),
            connection: RefCell::new(None),
            connect_status,
            wifi_status,
            mqtt_status,
        });
        let connect_listener =
            EventListener::new(connect_button.clone().into(), EventType::Click, {
                let setup_tab = Rc::downgrade(&setup_tab);
                move |_| {
                    if let Some(setup_tab) = setup_tab.upgrade() {
                        spawn_local(async move {
                            match BleConnection::new(
                                setup_tab.connect_status.clone(),
                                setup_tab.wifi_status.clone(),
                                setup_tab.mqtt_status.clone(),
                            )
                            .await
                            {
                                Ok(connection) => {
                                    *setup_tab.connection.borrow_mut() = Some(connection);
                                }
                                Err(e) => setup_tab.connect_status.set(
                                    StatusPriority::Error,
                                    format!("Failed to connect via bluetooth: {}", e),
                                ),
                            }
                        });
                    }
                    false
                }
            })?;
        setup_tab
            .connect_listener
            .set(connect_listener)
            .ok()
            .unwrap();
        let read_listener = EventListener::new(read_button.clone().into(), EventType::Click, {
            let setup_tab = Rc::downgrade(&setup_tab);
            move |_| {
                if let Some(setup_tab) = setup_tab.upgrade() {
                    spawn_local(async move {
                        if let Err(e) = setup_tab.read_settings().await {
                            setup_tab.connect_status.set(
                                StatusPriority::Error,
                                format!("Failed to read settings from bluetooth: {}", e),
                            );
                        }
                    });
                }
                false
            }
        })?;
        setup_tab.read_listener.set(read_listener).ok().unwrap();

        Ok(setup_tab)
    }
    async fn read_settings(&self) -> Result<(), Error> {
        if let Some(connection) = self.connection.borrow().clone() {
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
        }
        Ok(())
    }
}

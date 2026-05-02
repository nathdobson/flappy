use crate::browser_support::{BROWSER_SUPPORT_MESSAGE, check_usb_supported};
use crate::connection::{EitherClient, connect_usb};
use crate::error::Error;
use crate::event_listener::{EventListener, EventType};
use crate::status::{Status, StatusPriority};
use crate::tabs::TabContent;
use crate::utils::{AppendChild, create_element, sleep, try_window};
use empty_rc::EmptyRc;
use js_sys::futures::spawn_local;
use js_sys::{ArrayBuffer, Date, Uint8Array};
use log::info;
use picoboot::{Access, Picoboot};
use setup_client::client::Client;
use setup_client::flash_firmware::flash_firmware;
use std::rc::Rc;
use std::time::Duration;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlDivElement, HtmlElement, Response};

pub struct FirmwareTab {
    node: HtmlDivElement,
    firmware_listener: EventListener<'static>,
    firmware_status: Rc<Status>,
}

impl FirmwareTab {
    pub fn new() -> Result<Rc<Self>, Error> {
        let this = EmptyRc::<Self>::new();
        let node = create_element::<"div">()?;
        node.set_class_name("setup-tab");
        let firmware_section = node.append_element::<"div">()?;
        firmware_section.set_class_name("setup-section");
        let usb_supported = check_usb_supported().is_ok();
        if !usb_supported {
            firmware_section
                .append_element::<"p">()?
                .set_inner_html(BROWSER_SUPPORT_MESSAGE);
        }
        let firmware_button = firmware_section.append_element::<"button">()?;
        firmware_button.set_text_content(Some(&format!(
            "Flash firmware (version {})",
            crate::built_info::GIT_VERSION.unwrap_or("<unknown>")
        )));
        firmware_button.set_disabled(!usb_supported);
        let firmware_listener = EventListener::new(
            &firmware_button,
            EventType::Click,
            Self::weak_callback(&this, Self::flash_firmware),
        )?;
        let firmware_status = Status::new()?;
        firmware_section.append_child(firmware_status.node())?;
        Ok(this.into_rc(FirmwareTab {
            node,
            firmware_listener,
            firmware_status,
        }))
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
    fn flash_firmware(self: Rc<Self>, event: Event) {
        spawn_local(async move {
            self.firmware_status.reset();
            if let Err(e) = self.try_flash_firmware().await {
                self.firmware_status.set_error(StatusPriority::Error, &e);
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

        self.firmware_status
            .set(StatusPriority::Info, "Connecting...".to_string());

        let mut client = match connect_usb(self.firmware_status.clone()).await? {
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
        self.firmware_status.set(
            StatusPriority::Info,
            "Connecting to Picoboot...".to_string(),
        );
        let mut client = client.connect().await?;
        let start = Date::now();
        flash_firmware::<!>(&mut client, &binary, &mut |progress| {
            self.firmware_status
                .set(StatusPriority::Info, progress.to_string());
            Ok(())
        })
        .await?
        .into_ok();
        let end = Date::now();
        info!("Flashed firmware in {} seconds", (end - start) / 1000.0);
        self.firmware_status.set(
            StatusPriority::Info,
            "Successfully updated firmware!".to_string(),
        );
        Ok(())
    }
}

impl TabContent for FirmwareTab {
    fn title(&self) -> &str {
        "Update Firmware"
    }

    fn id(&self) -> &str {
        "firmware"
    }

    fn node(&self) -> &HtmlElement {
        &self.node
    }
}

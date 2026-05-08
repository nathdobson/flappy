use crate::browser_support::{
    BROWSER_SUPPORT_MESSAGE, check_usb_supported, force_webview_to_chrome,
};
use crate::connection::{EitherClient, connect_usb};
use crate::error::Error;
use crate::event_listener::{EventListener, EventType};
use crate::status::{Status, StatusPriority};
use crate::tabs::TabContent;
use crate::utils::{AppendChild, create_element, sleep, try_window};
use empty_rc::EmptyRc;
use js_sys::futures::spawn_local;
use js_sys::{ArrayBuffer, Date, Reflect, Uint8Array, eval, global};
use log::info;
use regex::Regex;
use setup_client::client::Client;
use setup_client::flash_firmware::flash_firmware;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlButtonElement, HtmlDivElement, HtmlElement, Response, window};

pub struct FirmwareTab {
    node: HtmlDivElement,
    firmware_button: HtmlButtonElement,
    firmware_status: Rc<Status>,
    #[allow(dead_code)]
    firmware_listener: EventListener<'static>,
}

impl FirmwareTab {
    pub fn new() -> Result<Rc<Self>, Error> {
        let this = EmptyRc::<Self>::new();
        let node = create_element::<"div">()?;
        node.set_class_name("setup-tab");
        let firmware_section = node.append_element::<"div">()?;
        firmware_section
            .append_element::<"p">()?
            .append_text(&format!(
                "Update firmware (version {})",
                crate::built_info::GIT_VERSION.unwrap_or("<unknown>")
            ))?;
        firmware_section.set_class_name("setup-section");

        let usb_supported = check_usb_supported().is_ok();
        if !usb_supported {
            force_webview_to_chrome()?;
            firmware_section
                .append_element::<"p">()?
                .set_inner_html(BROWSER_SUPPORT_MESSAGE);
        }
        firmware_section.append_text("Standard update process:")?;
        let list = firmware_section.append_element::<"ol">()?;
        list.append_element::<"li">()?
            .set_text_content(Some("Unplug the power supply from the display."));
        list.append_element::<"li">()?
            .set_text_content(Some("Connect this device to the display with a USB cable."));
        list.append_element::<"li">()?
            .append_text("Click 'Update Firmware'.")?;
        list.append_element::<"li">()?
            .append_text("Select 'Split Flap Display' in the first prompt.")?;
        list.append_element::<"li">()?
            .append_text("Select 'RP2350 Boot' in the second prompt.")?;
        list.append_element::<"li">()?
            .append_text("After updating, unplug the USB cable and plug in the power supply.")?;
        firmware_section.append_text("If the standard update process fails:")?;
        let list = firmware_section.append_element::<"ol">()?;
        list.append_element::<"li">()?
            .set_text_content(Some("Unplug the power supply from the display."));
        list.append_element::<"li">()?.set_text_content(Some(
            "Hold the white button on the display near the USB port.",
        ));
        list.append_element::<"li">()?
            .set_text_content(Some("Connect this device to the display with a USB cable."));
        list.append_element::<"li">()?
            .set_text_content(Some("Release the white button."));
        list.append_element::<"li">()?
            .append_text("Click 'Update Firmware'.")?;
        list.append_element::<"li">()?
            .append_text("Select 'RP2350 Boot'.")?;
        list.append_element::<"li">()?
            .append_text("After updating, unplug the USB cable and plug in the power supply.")?;
        let firmware_button = firmware_section.append_element::<"button">()?;
        firmware_button.set_text_content(Some("Update firmware"));
        firmware_button.set_disabled(!usb_supported);
        let firmware_listener = EventListener::new_weak(
            &firmware_button,
            EventType::Click,
            this.downgrade(),
            Self::flash_firmware,
        )?;
        let firmware_status = Status::new()?;
        firmware_section.append_child(firmware_status.node())?;
        Ok(this.into_rc(FirmwareTab {
            node,
            firmware_listener,
            firmware_status,
            firmware_button,
        }))
    }
    fn flash_firmware(self: Rc<Self>, _event: Event) {
        self.firmware_button.set_disabled(true);
        spawn_local(async move {
            self.firmware_status.reset();
            if let Err(e) = self.try_flash_firmware().await {
                self.firmware_status.set_error(
                    StatusPriority::Error,
                    "Failed to update firmware",
                    &e,
                );
            }
            self.firmware_button.set_disabled(false);
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

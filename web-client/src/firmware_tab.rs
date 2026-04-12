use crate::bind_weak::bind_weak_fn1;
use crate::error::Error;
use crate::event_listener::{EventListener, EventType};
use crate::status::{Status, StatusPriority};
use crate::tabs::TabContent;
use crate::utils::{create_element, document};
use crate::utils::{try_window, AppendChild};
use empty_rc::EmptyRc;
use js_sys::futures::spawn_local;
use js_sys::{ArrayBuffer, Uint8Array};
use log::info;
use picoboot::{Access, Picoboot};
use std::future::IntoFuture;
use std::rc::Rc;
use std::time::Duration;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Event, HtmlDivElement, HtmlElement, Response};

pub struct FirmwareTab {
    node: HtmlElement,
    status: Rc<Status>,
    listener: EventListener<'static>,
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

impl FirmwareTab {
    pub fn new() -> Result<Rc<Self>, Error> {
        let this = EmptyRc::<Self>::new();
        let node = create_element::<"ol">()?;
        //node.set_class_name("tab-contents");
        node.append_element::<"li">()?
            .set_text_content(Some("Unplug power and USB ports."));
        node.append_element::<"li">()?
            .set_text_content(Some("Hold white button on microcontroller."));
        node.append_element::<"li">()?
            .set_text_content(Some("Plug microcontroller into USB port of this machine."));
        let last = node.append_element::<"li">()?;
        let form = last.append_element::<"form">()?;

        let submit = form.append_element::<"input">()?;
        submit.set_type("submit");
        submit.set_value("Update Firmware");
        submit.set_class_name("firmware-button");
        let status = Status::new()?;
        node.append_child(status.node())?;
        let listener = EventListener::new(
            &form,
            EventType::Submit,
            bind_weak_fn1(this.downgrade(), |this, event: Event| {
                event.prevent_default();
                spawn_local(async move {
                    if let Err(e) = this.update_firmware().await {
                        this.status.set(StatusPriority::Error, format!("{}", e));
                    }
                });
                false
            }),
        )?;
        Ok(this.into_rc(FirmwareTab {
            node: node.into(),
            status,
            listener,
        }))
    }
    async fn update_firmware(&self) -> Result<(), Error> {
        self.status.set(
            StatusPriority::Info,
            "Retrieving firmware binary...".to_string(),
        );
        let binary: Response = try_window()?
            .fetch_with_str("./firmware.bin")
            .await?
            .dyn_into()?;
        let binary: ArrayBuffer = binary.array_buffer()?.into_future().await?.dyn_into()?;
        let binary = Uint8Array::new(&binary).to_vec();
        self.status.set(
            StatusPriority::Info,
            "Searching for USB device...".to_string(),
        );
        let mut picoboot = Picoboot::from_first(None).await?;
        self.status.set(
            StatusPriority::Info,
            "Connecting to USB device...".to_string(),
        );
        let conn = picoboot.connect().await?;
        self.status
            .set(StatusPriority::Info, "Resetting interface...".to_string());
        conn.reset_interface().await?;
        self.status.set(
            StatusPriority::Info,
            "Disabling mass storage...".to_string(),
        );
        conn.set_exclusive_access(Access::ExclusiveAndEject).await?;
        self.status
            .set(StatusPriority::Info, "Disabling XIP...".to_string());
        conn.exit_xip().await?;
        self.status
            .set(StatusPriority::Info, "Erasing flash...".to_string());
        conn.flash_erase_start(binary.len()).await?;
        self.status
            .set(StatusPriority::Info, "Writing firmware...".to_string());
        conn.flash_write_start(&binary).await?;
        self.status
            .set(StatusPriority::Info, "Verifying firmware...".to_string());
        let verified = conn.flash_read_start(binary.len() as u32).await?;
        if binary != verified {
            self.status.set(
                StatusPriority::Error,
                format!(
                    "firmware verification failed comparing {} bytes and {} bytes",
                    binary.len(),
                    verified.len()
                ),
            );
            return Ok(());
        }
        self.status
            .set(StatusPriority::Info, "Rebooting device...".to_string());
        conn.reboot(Duration::from_millis(500)).await?;
        self.status.set(
            StatusPriority::Info,
            "Firmware successfully updated!".to_string(),
        );
        Ok(())
    }
}

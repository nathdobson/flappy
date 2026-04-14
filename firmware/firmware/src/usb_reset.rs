use crate::error::Error;
use crate::make_static;
use crate::runtime::reboot_to_bootsel;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::control::{InResponse, OutResponse, Recipient, Request, RequestType};
use embassy_usb::types::{InterfaceNumber, StringIndex};
use embassy_usb::{Builder, Handler};
use protocol::usb::{
    CUSTOM_CLASS_ID, PICOBOOT_RESET_INTERFACE_PROTOCOL, PICOBOOT_RESET_REQUEST_BOOTSEL,
    PICOBOOT_SUBCLASS_ID,
};

/// Implement a USB interface for picotool to instruct the device to restart in BOOTSEL mode for
/// firmware updates.

pub struct UsbResetModule {}

struct UsbResetHandler {
    comm_if: InterfaceNumber,
    str_idx: StringIndex,
}

impl UsbResetModule {
    pub fn new() -> &'static Self {
        make_static!(UsbResetModule, UsbResetModule {})
    }
    pub async fn start(
        &self,
        spawner: Spawner,
        builder: &mut Builder<'static, Driver<'static, USB>>,
    ) -> Result<(), Error> {
        let str_idx = builder.string();
        let mut func = builder.function(
            CUSTOM_CLASS_ID,
            PICOBOOT_SUBCLASS_ID,
            PICOBOOT_RESET_INTERFACE_PROTOCOL,
        );
        let mut iface = func.interface();
        let comm_if = iface.interface_number();
        iface.alt_setting(
            CUSTOM_CLASS_ID,
            PICOBOOT_SUBCLASS_ID,
            PICOBOOT_RESET_INTERFACE_PROTOCOL,
            Some(str_idx),
        );
        drop(func);
        builder.handler(make_static!(
            UsbResetHandler,
            UsbResetHandler { comm_if, str_idx }
        ));
        Ok(())
    }
}

impl Handler for UsbResetHandler {
    fn control_out(&mut self, req: Request, data: &[u8]) -> Option<OutResponse> {
        if !(req.request_type == RequestType::Class
            && req.recipient == Recipient::Interface
            && req.index == u8::from(self.comm_if) as u16)
        {
            return None;
        }

        match req.request {
            PICOBOOT_RESET_REQUEST_BOOTSEL => {
                reboot_to_bootsel();
            }
            _ => Some(OutResponse::Rejected),
        }
    }

    fn control_in<'a>(&'a mut self, req: Request, buf: &'a mut [u8]) -> Option<InResponse<'a>> {
        if !(req.request_type == RequestType::Class
            && req.recipient == Recipient::Interface
            && req.index == u8::from(self.comm_if) as u16)
        {
            return None;
        }

        Some(InResponse::Rejected)
    }

    fn get_string(&mut self, index: StringIndex, lang_id: u16) -> Option<&str> {
        (index == self.str_idx).then_some("Reset")
    }
}

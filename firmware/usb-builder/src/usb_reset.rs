use crate::UsbServer;
use crate::error::Error;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::control::{InResponse, OutResponse, Recipient, Request, RequestType};
use embassy_usb::types::{InterfaceNumber, StringIndex};
use embassy_usb::{Builder, Handler};
use make_static::reexports::static_cell::StaticCell;
use protocol_usb::{
    CUSTOM_CLASS_ID, PICOBOOT_RESET_INTERFACE_PROTOCOL, PICOBOOT_RESET_REQUEST_BOOTSEL,
    PICOBOOT_SUBCLASS_ID,
};
use reboot::reboot_bootsel_after;

pub struct UsbResetServer {
    handler: StaticCell<UsbResetHandler>,
}

impl UsbResetServer {
    pub fn new() -> Self {
        UsbResetServer {
            handler: StaticCell::new(),
        }
    }
}

impl UsbServer for UsbResetServer {
    type ConfigDescBuffer = [u8; 128];
    type BosDescBuffer = [u8; 16];
    type MsosDescBuffer = [u8; 256];
    fn build(
        &'static self,
        _spawner: Spawner,
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
        builder.handler(self.handler.init(UsbResetHandler { comm_if, str_idx }));
        Ok(())
    }
}

struct UsbResetHandler {
    comm_if: InterfaceNumber,
    str_idx: StringIndex,
}

impl Handler for UsbResetHandler {
    fn control_out(&mut self, req: Request, _data: &[u8]) -> Option<OutResponse> {
        if !(req.request_type == RequestType::Class
            && req.recipient == Recipient::Interface
            && req.index == u8::from(self.comm_if) as u16)
        {
            return None;
        }

        match req.request {
            PICOBOOT_RESET_REQUEST_BOOTSEL => {
                reboot_bootsel_after(10);
                Some(OutResponse::Accepted)
            }
            _ => Some(OutResponse::Rejected),
        }
    }

    fn control_in<'a>(&'a mut self, req: Request, _buf: &'a mut [u8]) -> Option<InResponse<'a>> {
        if !(req.request_type == RequestType::Class
            && req.recipient == Recipient::Interface
            && req.index == u8::from(self.comm_if) as u16)
        {
            return None;
        }

        Some(InResponse::Rejected)
    }

    fn get_string(&mut self, index: StringIndex, _lang_id: u16) -> Option<&str> {
        (index == self.str_idx).then_some("Reset")
    }
}

fn assert_send_sync(x: UsbResetServer) -> impl Send + Sync {
    x
}

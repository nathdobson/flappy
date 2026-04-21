use crate::error::Error;
use crate::{MAX_PACKET_SIZE, UsbServer};
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Endpoint;
use embassy_rp::usb::Out;
use embassy_rp::usb::{Driver, In};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::once_lock::OnceLock;
use embassy_usb::Builder;
use embassy_usb_driver::Endpoint as _;
use embassy_usb_driver::EndpointIn;
use embassy_usb_driver::EndpointOut;
use heapless::VecView;
use log::{error};
use protocol_usb::{CUSTOM_CLASS_ID, CUSTOM_SUBCLASS_ID};

struct RpcEndpoints {
    in_ep: Endpoint<'static, USB, In>,
    out_ep: Endpoint<'static, USB, Out>,
}

pub struct UsbRpcServer {
    status_ep: OnceLock<Mutex<CriticalSectionRawMutex, Endpoint<'static, USB, In>>>,
    rpc_eps: OnceLock<Mutex<CriticalSectionRawMutex, RpcEndpoints>>,
}

fn assert_sync_send(x: UsbRpcServer) -> impl Sync + Send {
    x
}

impl UsbRpcServer {
    pub fn new() -> Self {
        Self {
            status_ep: OnceLock::new(),
            rpc_eps: OnceLock::new(),
        }
    }
    pub async fn receive_request(&self, data: &mut VecView<u8>) {
        data.resize(data.capacity(), 0).unwrap();
        let mut eps = self.rpc_eps.get().await.lock().await;
        eps.out_ep.wait_enabled().await;
        loop {
            match eps.out_ep.read_transfer(&mut *data).await {
                Ok(len) => {
                    data.resize(len, 0).unwrap();
                    return;
                }
                Err(e) => {
                    error!("Error receiving USB request {}", e);
                    eps.out_ep.wait_enabled().await;
                }
            };
        }
    }
    pub async fn send_response(&self, data: &[u8]) {
        let mut eps = self.rpc_eps.get().await.lock().await;
        if let Err(e) = eps.in_ep.write_transfer(data, true).await {
            error!("Error sending USB response {}", e);
        }
    }
    pub async fn send_status(&self, data: &[u8]) {
        let mut status_ep = self.status_ep.get().await.lock().await;
        if let Err(e) = status_ep.write_transfer(data, true).await {
            error!("Error sending USB status {}", e);
        }
    }
}

impl UsbServer for UsbRpcServer {
    type ConfigDescBuffer = [u8; 128];
    type BosDescBuffer = [u8; 16];
    type MsosDescBuffer = [u8; 256];

    fn build(
        &'static self,
        spawner: Spawner,
        builder: &mut Builder<'static, Driver<'static, USB>>,
    ) -> Result<(), Error> {
        let mut custom = builder.function(CUSTOM_CLASS_ID, CUSTOM_SUBCLASS_ID, 0x00);
        let mut custom_if = custom.interface();
        let mut alt = custom_if.alt_setting(CUSTOM_CLASS_ID, CUSTOM_SUBCLASS_ID, 0x00, None);
        let in_ep = alt.endpoint_bulk_in(None, MAX_PACKET_SIZE as u16);
        let out_ep = alt.endpoint_bulk_out(None, MAX_PACKET_SIZE as u16);
        let status_ep = alt.endpoint_bulk_in(None, MAX_PACKET_SIZE as u16);
        self.rpc_eps
            .init(Mutex::new(RpcEndpoints { in_ep, out_ep }))
            .ok()
            .unwrap();
        self.status_ep.init(Mutex::new(status_ep)).ok().unwrap();
        Ok(())
    }
}

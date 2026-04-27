use crate::error::Error;
use alloc::boxed::Box;
use core::cell::{Cell, RefCell};
use embassy_executor::{SendSpawner, Spawner};
use embassy_rp::Peri;
use embassy_rp::peripherals::USB;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embassy_sync::mutex::Mutex;
use embassy_sync::watch::Watch;
use fixed_freelist::{Freelist, FreelistStorage};
use heapless::{Vec, box_pool};
use log::{error, info};
use make_static::make_static;
use protocol::setup::{AppStatus, SetupRequest};
use protocol::setup::{MAX_SETUP_MESSAGE_SIZE, SetupResponse};
use protocol::{PRODUCT_MANUFACTURER, PRODUCT_NAME};
use runtime::{LocalSpawn, RemoteSpawn};
use usb_builder::usb_reset::UsbResetServer;
use usb_builder::usb_rpc::UsbRpcServer;
use usb_builder::usb_terminal::UsbTerminalServer;
use usb_builder::{UsbBuilder, UsbServer, UsbStack};
pub struct UsbModule {
    server: &'static FlappyUsbServer,
    status: Watch<NoopRawMutex, AppStatus, 1>,
}

#[derive(UsbServer)]
pub struct FlappyUsbServer {
    usb_reset_server: UsbResetServer,
    usb_terminal: UsbTerminalServer,
    #[cfg(feature = "setup")]
    usb_rpc: UsbRpcServer,
}

impl FlappyUsbServer {
    pub fn new(spawner: SendSpawner, peri: Peri<'static, USB>) -> &'static Self {
        let server: &FlappyUsbServer = make_static!(
            FlappyUsbServer,
            FlappyUsbServer {
                usb_terminal: UsbTerminalServer::new(),
                usb_reset_server: UsbResetServer::new(),
                #[cfg(feature = "setup")]
                usb_rpc: UsbRpcServer::new(),
            }
        );
        server.usb_terminal.set_logger();
        make_static!(_, RemoteSpawn::new(spawner)).spawn(move |spawner| async move {
            if let Err(e) = server.start(spawner, peri).await {
                info!("uncaught runtime error: {:?}", e);
            }
        });
        server
    }
    async fn start(&'static self, spawner: Spawner, peri: Peri<'static, USB>) -> Result<(), Error> {
        let stack = make_static!(UsbStack<FlappyUsbServer>, UsbStack::new());
        UsbBuilder {
            server: self,
            stack,
            peri,
            spawner,
            manufacturer: Some(PRODUCT_MANUFACTURER),
            product: Some(PRODUCT_NAME),
        }
        .build()?;
        Ok(())
    }
}

impl UsbModule {
    pub fn new(spawner: Spawner, server: &'static FlappyUsbServer) -> &'static Self {
        let module: &_ = make_static!(
            UsbModule,
            UsbModule {
                server,
                status: Watch::new(),
            }
        );
        module.server.usb_terminal.set_logger();
        #[cfg(feature = "setup")]
        make_static!(_, LocalSpawn::new(spawner)).spawn(move || async move {
            let mut receiver = module.status.receiver().unwrap();
            loop {
                let status = receiver.changed().await;
                match serde_json_core::to_vec::<_, MAX_SETUP_MESSAGE_SIZE>(&status) {
                    Ok(buffer) => {
                        module.server.usb_rpc.send_status(&buffer).await;
                    }
                    Err(e) => {
                        error!("Failed to serialize status: {}", e);
                    }
                }
            }
        });
        module
    }
    pub fn terminal(&'static self) -> &'static UsbTerminalServer {
        &self.server.usb_terminal
    }

    pub fn update_status<F: FnOnce(&mut AppStatus)>(&'static self, f: F) {
        let mut f = Cell::new(Some(f));
        self.status
            .sender()
            .send_modify(move |x| f.take().unwrap()(x.get_or_insert_default()))
    }

    #[cfg(feature = "setup")]
    pub async fn receive_request(&self) -> SetupRequest {
        loop {
            let mut buffer = Vec::<u8, MAX_SETUP_MESSAGE_SIZE>::new();
            let mut tmp = Vec::<u8, MAX_SETUP_MESSAGE_SIZE>::new();
            self.server.usb_rpc.receive_request(&mut buffer).await;
            match serde_json_core::from_slice_escaped(&buffer, &mut tmp) {
                Ok(request) => return request.0,
                Err(e) => error!("Error deserializing request: {}", e),
            }
        }
    }
    #[cfg(feature = "setup")]
    pub async fn send_response(&self, response: &SetupResponse) {
        let buffer = serde_json_core::to_vec::<_, MAX_SETUP_MESSAGE_SIZE>(response).unwrap();
        self.server.usb_rpc.send_response(buffer.as_slice()).await;
    }
}

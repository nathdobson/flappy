use crate::error::{Error, Result};
use crate::wifi_proto::{WifiSettings, WifiStatus};
use core::cell::RefCell;
use core::fmt::{Display, Formatter};
use core::str::from_utf8;
use cyw43::{JoinOptions, NetDriver};
use embassy_executor::Spawner;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::{Config, Stack, StackResources};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use log::{error, info, warn};
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use serde_json_core::from_slice;
use static_cell::make_static;
use trouble_host::prelude::HeaplessString;
use crate::radio::RadioModule;

const MODULE: &'static str = "[WiFi ]";
struct WifiStatusBuilder {
    link_up: bool,
    dhcp_up: bool,
}

pub struct WifiModule {
    spawner: Spawner,
    stack: Stack<'static>,
    radio: &'static RadioModule,
    settings: Signal<NoopRawMutex, WifiSettings>,
    status: RefCell<WifiStatusBuilder>,
}

pub trait WifiHandler {
    fn handle_wifi_status(&self, status: WifiStatus);
}

impl WifiStatusBuilder {
    pub fn build(&self) -> WifiStatus {
        if self.link_up {
            if self.dhcp_up {
                WifiStatus::DhcpUp
            } else {
                WifiStatus::LinkUp
            }
        } else {
            WifiStatus::Disconnected
        }
    }
}

impl WifiModule {
    pub async fn new(
        spawner: Spawner,
        radio: &'static RadioModule,
        net_device: NetDriver<'static>,
        rng: &mut impl RngCore,
    ) -> Result<&'static WifiModule> {
        info!("{MODULE} Starting WiFi");
        let config = Config::dhcpv4(Default::default());
        let seed = rng.next_u64();
        let resources = make_static!(StackResources::<5>::new());
        let (stack, runner) = embassy_net::new(net_device, config, resources, seed);
        spawner.spawn({
            #[embassy_executor::task]
            async fn run_runner(
                mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>,
            ) -> ! {
                runner.run().await
            }
            run_runner(runner)?
        });
        let module = make_static!(WifiModule {
            spawner,
            stack,
            radio,
            settings: Signal::new(),
            status: RefCell::new(WifiStatusBuilder {
                link_up: false,
                dhcp_up: false,
            }),
        });
        spawner.spawn({
            #[embassy_executor::task]
            async fn connect_to_wifi(module: &'static WifiModule) {
                module.connect_to_wifi().await
            }
            connect_to_wifi(module)?
        });
        info!("{MODULE} Started");
        Ok(module)
    }
    pub fn set_settings(&self, settings: WifiSettings) {
        self.settings.signal(settings);
    }
    pub fn stack(&'static self) -> &'static Stack<'static> {
        &self.stack
    }
    async fn update_link_status(&'static self, handler: &'static dyn WifiHandler) {
        loop {
            self.stack.wait_link_up().await;
            self.status.borrow_mut().link_up = true;
            handler.handle_wifi_status(self.status.borrow().build());
            self.stack.wait_link_down().await;
            self.status.borrow_mut().link_up = false;
            handler.handle_wifi_status(self.status.borrow().build());
        }
    }

    async fn update_dhcp_status(&'static self, handler: &'static dyn WifiHandler) {
        loop {
            self.stack.wait_config_up().await;
            self.status.borrow_mut().dhcp_up = true;
            handler.handle_wifi_status(self.status.borrow().build());
            self.stack.wait_config_down().await;
            self.status.borrow_mut().dhcp_up = true;
            handler.handle_wifi_status(self.status.borrow().build());
        }
    }

    async fn connect_to_wifi(&'static self) {
        let mut settings = self.settings.wait().await;
        loop {
            if let Some(new) = self.settings.try_take() {
                settings = new;
            }
            info!(
                "{MODULE} Connecting to WiFi network with SSID {:?} and password {:?}",
                settings.ssid, settings.password
            );
            while let Err(err) = self
                .radio
                .control
                .lock()
                .await
                .join(
                    &settings.ssid,
                    JoinOptions::new(settings.password.as_bytes()),
                )
                .await
            {
                warn!("{MODULE} Failed to join WiFi network ({})", err.status);
                continue;
            }
            info!("{MODULE} Connected to WiFi network");
            settings = self.settings.wait().await;
        }
    }

    pub fn start(&'static self, handler: &'static dyn WifiHandler) -> Result<()> {
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn update_link_status(
                module: &'static WifiModule,
                handler: &'static dyn WifiHandler,
            ) {
                module.update_link_status(handler).await;
            }
            update_link_status(self, handler)?
        });
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn update_dhcp_status(
                module: &'static WifiModule,
                handler: &'static dyn WifiHandler,
            ) {
                module.update_dhcp_status(handler).await;
            }
            update_dhcp_status(self, handler)?
        });
        Ok(())
    }
}

impl Display for WifiStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            WifiStatus::Disconnected => write!(f, "Disconnected"),
            WifiStatus::LinkUp => write!(f, "Waiting for IP Address"),
            WifiStatus::DhcpUp => write!(f, "Connected"),
        }
    }
}

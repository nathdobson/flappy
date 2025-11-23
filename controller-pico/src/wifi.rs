use crate::error::{Error, Result};
use crate::led::LedModule;
use crate::radio::RadioModule;
use core::cell::RefCell;
use core::fmt::{Display, Formatter};
use core::str::from_utf8;
use cyw43::{Control, JoinOptions, NetDriver};
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
use static_cell::StaticCell;
use trouble_host::prelude::HeaplessString;

pub struct WifiModuleBuilder<'build, R> {
    pub spawner: Spawner,
    pub rng: &'build mut R,
    pub net_device: NetDriver<'static>,
    pub radio: &'static RadioModule,
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

struct WifiStatusBuilder {
    link_up: bool,
    dhcp_up: bool,
}

pub enum WifiStatus {
    Disconnected,
    LinkUp,
    DhcpUp,
}

pub struct WifiModule {
    pub stack: Stack<'static>,
    pub radio: &'static RadioModule,
    settings: Signal<NoopRawMutex, WifiSettings>,
    status: RefCell<WifiStatusBuilder>,
}

pub struct WifiTask {
    module: &'static WifiModule,
}

pub trait WifiHandler {
    fn handle_status(&self, status: WifiStatus);
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct WifiSettings {
    pub ssid: HeaplessString<32>,
    pub password: HeaplessString<63>,
}

impl WifiTask {
    pub fn spawn(&self, spawner: Spawner, handler: &'static dyn WifiHandler) -> Result<()> {
        spawner.spawn(update_link_status(self.module, handler)?);
        spawner.spawn(update_dhcp_status(self.module, handler)?);
        spawner.spawn(connect_to_wifi(self.module)?);
        Ok(())
    }
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

#[embassy_executor::task]
async fn update_link_status(module: &'static WifiModule, handler: &'static dyn WifiHandler) {
    loop {
        module.stack.wait_link_up().await;
        module.status.borrow_mut().link_up = true;
        handler.handle_status(module.status.borrow().build());
        module.stack.wait_link_down().await;
        module.status.borrow_mut().link_up = false;
        handler.handle_status(module.status.borrow().build());
    }
}

#[embassy_executor::task]
async fn update_dhcp_status(module: &'static WifiModule, handler: &'static dyn WifiHandler) {
    loop {
        module.stack.wait_config_up().await;
        module.status.borrow_mut().dhcp_up = true;
        handler.handle_status(module.status.borrow().build());
        module.stack.wait_config_down().await;
        module.status.borrow_mut().dhcp_up = true;
        handler.handle_status(module.status.borrow().build());
    }
}

#[embassy_executor::task]
async fn connect_to_wifi(module: &'static WifiModule) {
    let mut settings = module.settings.wait().await;
    loop {
        if let Some(new) = module.settings.try_take() {
            settings = new;
        }
        info!("Connecting to wifi {:?}", settings);
        while let Err(err) = module
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
            warn!("[WIFI] join failed with status={}", err.status);
            continue;
        }
        settings = module.settings.wait().await;
    }
}

impl WifiModule {
    pub fn set_settings(&self, settings: WifiSettings) {
        self.settings.signal(settings);
    }
}

impl<'build, R: RngCore> WifiModuleBuilder<'build, R> {
    pub async fn build(mut self) -> Result<(WifiTask, &'static WifiModule)> {
        let config = Config::dhcpv4(Default::default());
        let seed = self.rng.next_u64();

        // Init network stack
        static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
        let (stack, runner) = embassy_net::new(
            self.net_device,
            config,
            RESOURCES.init(StackResources::new()),
            seed,
        );

        self.spawner.spawn(net_task(runner)?);

        static MODULE: StaticCell<WifiModule> = StaticCell::new();
        let module = MODULE.init(WifiModule {
            stack,
            radio: self.radio,
            settings: Signal::new(),
            status: RefCell::new(WifiStatusBuilder {
                link_up: false,
                dhcp_up: false,
            }),
        });
        Ok((WifiTask { module }, module))
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

use crate::error::{Error, Result};
use crate::led::LedModule;
use crate::radio::RadioModule;
use crate::secrets::{WIFI_NETWORK, WIFI_PASSWORD};
use core::cell::RefCell;
use core::str::from_utf8;
use cyw43::{Control, JoinOptions, NetDriver};
use embassy_executor::Spawner;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::{Config, Stack, StackResources};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
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

pub struct WifiModule {
    pub stack: Stack<'static>,
}

pub struct WifiTask {
    module: &'static WifiModule,
}

pub trait WifiHandler {
    fn handle_link_status(&self, state: bool);
    fn handle_dhcp_status(&self, state: bool);
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct WifiSettings {
    pub ssid: HeaplessString<32>,
    pub password: HeaplessString<63>,
}

impl WifiTask {
    pub fn spawn(&self, spawner: Spawner, handler: &'static dyn WifiHandler) -> Result<()> {
        spawner.spawn(update_link_status(self.module, handler)?);
        spawner.spawn(update_dhcp_status(self.module, handler)?);
        Ok(())
    }
}

#[embassy_executor::task]
async fn update_link_status(module: &'static WifiModule, handler: &'static dyn WifiHandler) {
    loop {
        module.stack.wait_link_up().await;
        handler.handle_link_status(true);
        module.stack.wait_link_down().await;
        handler.handle_link_status(false);
    }
}

#[embassy_executor::task]
async fn update_dhcp_status(module: &'static WifiModule, handler: &'static dyn WifiHandler) {
    loop {
        module.stack.wait_config_up().await;
        handler.handle_dhcp_status(true);
        module.stack.wait_config_down().await;
        handler.handle_dhcp_status(false);
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

        while let Err(err) = self
            .radio
            .control
            .lock()
            .await
            .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD))
            .await
        {
            warn!("[WIFI] join failed with status={}", err.status);
        }

        static MODULE: StaticCell<WifiModule> = StaticCell::new();
        let module = MODULE.init(WifiModule { stack });
        Ok((WifiTask { module }, module))
    }
}

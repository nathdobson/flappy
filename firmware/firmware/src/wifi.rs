use crate::error::{Error, Result};
use crate::radio::RadioModule;
use core::cell::RefCell;
use core::fmt::{Display, Formatter};
use core::str::from_utf8;
use cyw43::{JoinOptions, NetDriver};
use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_sync::watch::{DynReceiver, Watch};
use embassy_time::{Duration, Timer};
use log::{error, info, warn};
use protocol::setup::{WifiSettings, WifiStatus};
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use crate::make_static;

const MODULE: &'static str = "[WiFi ]";

pub struct WifiModule {
    spawner: Spawner,
    stack: Stack<'static>,
    radio: &'static RadioModule,
    settings: Signal<NoopRawMutex, WifiSettings>,
    status: Watch<NoopRawMutex, WifiStatus, 1>,
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
        let resources = make_static!(StackResources::<5>, StackResources::new());
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
        let module = make_static!(WifiModule, WifiModule {
            spawner,
            stack,
            radio,
            settings: Signal::new(),
            status: Watch::new(),
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
    async fn connect_to_wifi(&'static self) {
        let mut settings = self.settings.wait().await;
        loop {
            if let Some(new) = self.settings.try_take() {
                settings = new;
            }
            if settings.ssid.is_empty() {
                self.status.sender().send(WifiStatus::Unconfigured);
                info!("{MODULE} WiFi not configured.");
                settings = self.settings.wait().await;
            }
            self.status.sender().send_if_modified(|x| {
                let x = x.get_or_insert_default();
                match x {
                    WifiStatus::Unconfigured => {
                        *x = WifiStatus::Disconnected;
                        true
                    }
                    _ => false,
                }
            });
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
                warn!("{MODULE} Failed to join WiFi network ({:?})", err);
                continue;
            }
            self.status.sender().send(WifiStatus::Connected);
            info!("{MODULE} Connected to WiFi network");
            settings = self.settings.wait().await;
        }
    }
    pub fn watch_status(&'static self) -> Option<DynReceiver<'static, WifiStatus>> {
        self.status.dyn_receiver()
    }
}

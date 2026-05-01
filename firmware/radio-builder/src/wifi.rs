use crate::Error;
use cyw43::{Control, JoinOptions};
use embassy_executor::Spawner;
use embassy_executor::raw::TaskPool;
use embassy_net::{Stack, StackResources};
use embassy_rp::clocks::RoscRng;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_sync::watch;
use embassy_sync::watch::Watch;
use embassy_time::Delay;
use embedded_hal_async::delay::DelayNs;
use log::{info, warn};
use make_static::make_static;
use protocol_wifi::{WifiSettings, WifiStatus};
use static_cell::StaticCell;
pub struct WifiPeripherals {
    pub(crate) net: cyw43::NetDriver<'static>,
    pub(crate) control: &'static Mutex<NoopRawMutex, Control<'static>>,
}

pub struct WifiStack<const SOCK: usize> {
    resources: StackResources<SOCK>,
    runner_pool: StaticCell<TaskPool<RunnerRun, 1>>,
    connect_pool: StaticCell<TaskPool<ConnectToWifi, 1>>,
}

pub struct WifiBuilder<const SOCK: usize> {
    pub spawner: Spawner,
    pub peripherals: WifiPeripherals,
    pub stack: &'static mut WifiStack<SOCK>,
}

pub struct Wifi {
    settings: Signal<NoopRawMutex, WifiSettings>,
    status: Watch<NoopRawMutex, WifiStatus, 1>,
    control: &'static Mutex<NoopRawMutex, Control<'static>>,
    stack: Stack<'static>,
}

type RunnerRun = impl Future<Output = !>;

#[define_opaque(RunnerRun)]
fn runner_run(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> RunnerRun {
    async move { runner.run().await }
}

type ConnectToWifi = impl Future<Output = !>;
#[define_opaque(ConnectToWifi)]
fn connect_to_wifi(wifi: &'static Wifi) -> ConnectToWifi {
    wifi.connect_to_wifi()
}

impl<const SOCK: usize> WifiStack<SOCK> {
    pub fn new() -> Self {
        WifiStack {
            resources: StackResources::new(),
            runner_pool: StaticCell::new(),
            connect_pool: StaticCell::new(),
        }
    }
}

impl<const SOCK: usize> WifiBuilder<SOCK> {
    pub fn build(self) -> Result<&'static Wifi, Error> {
        info!("Starting WiFi");
        let config = embassy_net::Config::dhcpv4(Default::default());
        let seed = RoscRng.next_u64();
        let resources = &mut self.stack.resources;
        let (stack, runner) = embassy_net::new(self.peripherals.net, config, resources, seed);
        let module: &_ = make_static!(
            Wifi,
            Wifi {
                settings: Signal::new(),
                status: Watch::new(),
                control: self.peripherals.control,
                stack,
            }
        );
        self.spawner.spawn(
            self.stack
                .runner_pool
                .init_with(TaskPool::new)
                .spawn(|| runner_run(runner))?,
        );
        self.spawner.spawn(
            self.stack
                .connect_pool
                .init_with(TaskPool::new)
                .spawn(|| connect_to_wifi(module))?,
        );
        Ok(module)
    }
}

impl Wifi {
    async fn connect_to_wifi(&self) -> ! {
        let mut settings = self.settings.wait().await;
        loop {
            if let Some(new) = self.settings.try_take() {
                settings = new;
            }
            if settings.ssid.is_empty() {
                self.status.sender().send(WifiStatus::Unconfigured);
                info!("WiFi not configured.");
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
                "Connecting to WiFi network with SSID {:?} and password {:?}",
                settings.ssid, settings.password
            );
            while let Err(err) = self
                .control
                .lock()
                .await
                .join(
                    &settings.ssid,
                    JoinOptions::new(settings.password.as_bytes()),
                )
                .await
            {
                warn!("Failed to join WiFi network ({:?})", err);
                Delay.delay_ms(1000).await;
                continue;
            }
            self.status.sender().send(WifiStatus::Connected);
            info!("Connected to WiFi network");
            settings = self.settings.wait().await;
        }
    }
    pub fn stack(&'static self) -> &'static Stack<'static> {
        &self.stack
    }
    pub fn watch_status(&'static self) -> Option<watch::DynReceiver<'static, WifiStatus>> {
        self.status.dyn_receiver()
    }
    pub fn set_settings(&self, settings: WifiSettings) {
        self.settings.signal(settings);
    }
}

use crate::error::Error;
use crate::flash_proto::FlashSettings;
use crate::peripherals::AppPeripherals;
use crate::product::{built_info, serial_number};
use crate::runtime::RuntimeModule;
use crate::wifi_proto::WifiStatus;
use core::cell::RefCell;
use core::future::pending;
use embassy_executor::Spawner;
use embassy_rp::clocks::RoscRng;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use heapless::{String, format};
use log::{error, info};
use static_cell::make_static;

pub const MODULE: &'static str = "[APP  ]";
pub struct Application {
    spawner: Spawner,
    runtime: &'static RuntimeModule,
    #[cfg(feature = "flash")]
    flash: &'static crate::flash::FlashModule,
    #[cfg(feature = "radio")]
    ble: &'static crate::ble::BleModule,
    #[cfg(feature = "radio")]
    led: &'static crate::led::LedModule,
    #[cfg(feature = "radio")]
    wifi: &'static crate::wifi::WifiModule,
    #[cfg(feature = "radio")]
    mqtt: &'static crate::mqtt::MqttModule,
    #[cfg(feature = "display")]
    driver: &'static crate::driver::DriverModule,
    state: RefCell<FlashSettings>,
    wifi_status: Signal<NoopRawMutex, WifiStatus>,
}

fn trim_null<const N: usize>(mut x: String<N>) -> String<N> {
    if x.as_bytes().last() == Some(&b'\0') {
        x.pop();
    }
    x
}

#[cfg(feature = "radio")]
impl crate::ble::BleHandler for Application {
    fn ble_handle_gatt_write(&self, id: u16) {
        let service = &self.ble.server().flappy_service;
        let ref mut state = *self.state.borrow_mut();
        let mut updated = false;
        if id == service.wifi_password.handle {
            updated = true;
            state.wifi.ssid = trim_null(self.ble.get(&service.wifi_ssid).unwrap_or_default());
            state.wifi.password =
                trim_null(self.ble.get(&service.wifi_password).unwrap_or_default());
            self.wifi.set_settings(state.wifi.clone());
            info!("{MODULE} Updating WiFi settings");
        } else if id == service.mqtt_topic.handle {
            updated = true;
            state.mqtt.hostname =
                trim_null(self.ble.get(&service.mqtt_hostname).unwrap_or_default());
            let port = trim_null(self.ble.get(&service.mqtt_port).unwrap_or_default());
            let port = &port;
            let port = port.strip_prefix("\"").unwrap_or(port);
            let port = port.strip_suffix("\"").unwrap_or(port);
            let port: u16 = port.parse().unwrap_or_default();
            state.mqtt.port = port;
            state.mqtt.username =
                trim_null(self.ble.get(&service.mqtt_username).unwrap_or_default());
            state.mqtt.password =
                trim_null(self.ble.get(&service.mqtt_password).unwrap_or_default());
            state.mqtt.topic = trim_null(self.ble.get(&service.mqtt_topic).unwrap_or_default());
            self.mqtt.set_settings(state.mqtt.clone());
            info!("{MODULE} Updating MQTT settings");
        }
        if updated {
            #[cfg(feature = "flash")]
            if let Err(e) = self.flash.save(state) {
                error!("{MODULE} failed to update wifi settings in flash {}", e);
            }
        }
    }
}

#[cfg(feature = "radio")]
impl crate::wifi::WifiHandler for Application {
    fn handle_wifi_status(&self, status: WifiStatus) {
        self.wifi_status.signal(status);
    }
}

impl Application {
    async fn new(
        spawner: Spawner,
        runtime: &'static RuntimeModule,
        peri: AppPeripherals,
    ) -> Result<&'static Self, Error> {
        #[cfg(feature = "display")]
        let driver = crate::driver::DriverModule::new(peri.driver_peri).await?;
        // for i in 0.. {
        //     let x = driver.count().ok();
        //     if i % 100 == 0 {
        //         info!("{:?}", x);
        //     }
        //     Timer::after_millis(100).await;
        // }
        #[cfg(feature = "display")]
        driver.write(&[0; 128])?;
        let mut rng = RoscRng;
        #[cfg(feature = "flash")]
        let flash = crate::flash::FlashModule::new(peri.flash_peri).await?;
        #[cfg(feature = "radio")]
        let (radio, bt_device, net_device) =
            crate::radio::RadioModule::new(spawner, peri.radio_peri).await?;
        #[cfg(feature = "radio")]
        let led = crate::led::LedModule::new(spawner, radio).await?;
        #[cfg(feature = "radio")]
        let ble = crate::ble::BleModule::new(spawner, bt_device).await?;
        #[cfg(feature = "radio")]
        let wifi = crate::wifi::WifiModule::new(spawner, radio, net_device, &mut rng).await?;
        #[cfg(feature = "radio")]
        let mqtt = crate::mqtt::MqttModule::new(spawner, &wifi.stack()).await?;
        #[cfg(feature = "flash")]
        let state = flash.load().await?;
        #[cfg(not(feature = "flash"))]
        let state = FlashSettings::default();
        let application = make_static!(Application {
            spawner,
            runtime,
            #[cfg(feature = "flash")]
            flash,
            #[cfg(feature = "radio")]
            ble,
            #[cfg(feature = "radio")]
            wifi,
            #[cfg(feature = "radio")]
            led,
            #[cfg(feature = "radio")]
            mqtt,
            #[cfg(feature = "display")]
            driver,
            state: RefCell::new(state.clone()),
            wifi_status: Signal::new(),
        });
        Ok(application)
    }
    fn initialize_settings(&'static self) -> Result<(), Error> {
        #[cfg(feature = "radio")]
        {
            let state = self.state.borrow();
            let service = &self.ble.server().flappy_service;
            self.ble.set(&service.wifi_ssid, &state.wifi.ssid);
            self.ble.set(&service.wifi_password, &state.wifi.password);
            self.ble.set(&service.mqtt_hostname, &state.mqtt.hostname);
            self.ble
                .set(&service.mqtt_port, &format!("\"{}\"", &state.mqtt.port)?);
            self.ble.set(&service.mqtt_username, &state.mqtt.username);
            self.ble.set(&service.mqtt_password, &state.mqtt.password);
            self.ble.set(&service.mqtt_topic, &state.mqtt.topic);
            self.wifi.set_settings(state.wifi.clone());
            self.mqtt.set_settings(state.mqtt.clone());
        }
        Ok(())
    }
    fn spawn_tasks(&'static self) -> Result<(), Error> {
        #[cfg(feature = "radio")]
        self.ble.start(self)?;
        #[cfg(feature = "radio")]
        self.wifi.start(self)?;
        #[cfg(feature = "radio")]
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn notify_mqtt_status(application: &'static Application) {
                application.notify_mqtt_status().await;
            }
            notify_mqtt_status(self)?
        });
        #[cfg(feature = "radio")]
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn notify_wifi_status(application: &'static Application) {
                application.notify_wifi_status().await;
            }
            notify_wifi_status(self)?
        });
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn display_message(application: &'static Application) {
                application.display_message().await;
            }
            display_message(self)?
        });
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn handle_commands(application: &'static Application) {
                application.handle_commands().await;
            }
            handle_commands(self)?
        });
        Ok(())
    }
    #[cfg(feature = "radio")]
    async fn notify_mqtt_status(&'static self) {
        loop {
            let status = self.mqtt.status().wait().await;
            self.ble
                .set_and_notify(
                    &self.ble.server().flappy_service.mqtt_status,
                    &format!("{}", status).unwrap_or_default(),
                )
                .await;
        }
    }
    #[cfg(feature = "radio")]
    async fn notify_wifi_status(&'static self) {
        loop {
            let status = self.wifi_status.wait().await;

            self.ble
                .set_and_notify(
                    &self.ble.server().flappy_service.wifi_status,
                    &format!("{}", status).unwrap_or_default(),
                )
                .await;
        }
    }
    async fn display_message(&'static self) {
        #[cfg(feature = "display")]
        let mut display = crate::display::Display::new(self.driver);
        // if let Err(e) = display.run("").await {
        //     error!("{MODULE} error when resetting flaps: {}", e);
        // }
        // self.mqtt.send(FlappyResponse::Start);
        #[cfg(feature = "radio")]
        loop {
            let request = self.mqtt.receive().wait().await;
            match request {
                proto::FlappyRequest::Run(msg) => {
                    info!("{MODULE} Displaying {}", msg);
                    self.mqtt.send(proto::FlappyResponse::Start(msg.clone()));
                    // Timer::after_millis(1000).await;
                    #[cfg(feature = "display")]
                    if let Err(e) = display.run(&msg).await {
                        error!("{MODULE} error when displaying message: {:?}", e);
                    }
                    self.mqtt.send(proto::FlappyResponse::Stop(msg.clone()));
                }
            }
        }
    }
    async fn handle_commands(&'static self) {
        loop {
            let command = self.runtime.commands().receive().await;
            if let Ok(s) = str::from_utf8(&command) {
                self.runtime.write_feedback_line(format_args!("{}", s)).await;
            }
            info!("Command = {:?}", command);
        }
    }
}

#[embassy_executor::task]
pub async fn main_task(spawner: Spawner, runtime: &'static RuntimeModule, peri: AppPeripherals) {
    if let Result::<(), Error>::Err(e) = try {
        info!("{MODULE} Welcome to the 3D printed Split Flap Display!");
        info!(
            "{MODULE} GIT_VERSION: {}",
            built_info::GIT_VERSION.unwrap_or("<unknown>")
        );
        info!(
            "{MODULE} GIT_DIRTY: {}",
            built_info::GIT_DIRTY.unwrap_or(false)
        );
        info!(
            "{MODULE} GIT_HEAD_REF: {}",
            built_info::GIT_HEAD_REF.unwrap_or("<unknown>")
        );
        if let Some(sn) = serial_number() {
            info!("{MODULE} MCU Serial Number: {}", sn);
        }

        let app = Application::new(spawner, runtime, peri).await?;
        app.initialize_settings()?;
        app.spawn_tasks()?;
        pending::<!>().await;
    } {
        error!("{MODULE} Uncaught error: {:?}", e);
    }
}

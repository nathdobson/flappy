use crate::cli::{Adjustment, Command, MqttField, WifiField};
use crate::error::Error;
use crate::peripherals::AppPeripherals;
use crate::product;
use crate::product::{built_info, serial_number};
use crate::runtime::RuntimeModule;
use core::cell::RefCell;
use core::future::pending;
use core::mem;
use core::num::ParseIntError;
use embassy_executor::Spawner;
use embassy_rp::clocks::RoscRng;
use embassy_rp::otp::get_chipid;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::{Channel, DynamicReceiver, DynamicSender};
use embassy_sync::signal::Signal;
use embassy_sync::watch::Watch;
use embassy_time::Timer;
use glyph_list::LETTERS;
use heapless::{CapacityError, String, Vec, format};
use log::{error, info};
use protocol::display::MAX_GLYPH_BYTES;
use protocol::display::MAX_GLYPHS;
use protocol::display::{DisplayRequest, DisplayResponse};
use protocol::setup::{
    AppSettings, AppStatus, DeviceInfo, SetupRequest, SetupResponse, WriteSettingsError,
};
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
    #[cfg(feature = "display")]
    display: &'static crate::display::DisplayModule,
    display_request: &'static Signal<NoopRawMutex, DisplayRequest>,
    display_response: &'static Channel<NoopRawMutex, DisplayResponse, 1>,
    settings: RefCell<AppSettings>,
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
        todo!();
        // let service = &self.ble.server().flappy_service;
        // let ref mut state = self.settings.borrow().clone();
        // let mut updated = false;
        // if id == service.wifi_password.handle {
        //     updated = true;
        //     state.wifi.ssid = trim_null(self.ble.get(&service.wifi_ssid).unwrap_or_default());
        //     state.wifi.password =
        //         trim_null(self.ble.get(&service.wifi_password).unwrap_or_default());
        //     self.wifi.set_settings(state.wifi.clone());
        //     info!("{MODULE} Updating WiFi settings");
        // } else if id == service.mqtt_topic.handle {
        //     updated = true;
        //     state.mqtt.hostname =
        //         trim_null(self.ble.get(&service.mqtt_hostname).unwrap_or_default());
        //     let port = trim_null(self.ble.get(&service.mqtt_port).unwrap_or_default());
        //     let port = &port;
        //     let port = port.strip_prefix("\"").unwrap_or(port);
        //     let port = port.strip_suffix("\"").unwrap_or(port);
        //     let port: u16 = port.parse().unwrap_or_default();
        //     state.mqtt.port = port;
        //     state.mqtt.username =
        //         trim_null(self.ble.get(&service.mqtt_username).unwrap_or_default());
        //     state.mqtt.password =
        //         trim_null(self.ble.get(&service.mqtt_password).unwrap_or_default());
        //     state.mqtt.topic = trim_null(self.ble.get(&service.mqtt_topic).unwrap_or_default());
        //     self.mqtt.set_settings(state.mqtt.clone());
        //     info!("{MODULE} Updating MQTT settings");
        // }
        // if updated {
        //     #[cfg(feature = "flash")]
        //     if let Err(e) = self.flash.save(state) {
        //         error!("{MODULE} failed to update settings in flash {}", e);
        //     }
        // }
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
        #[cfg(feature = "display")]
        driver.write(&[0; 128])?;
        #[cfg(feature = "display")]
        let mut display = crate::display::DisplayModule::new(driver);
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
        let display_request: &'static Signal<NoopRawMutex, DisplayRequest> =
            make_static!(Signal::new());
        let display_response: &'static Channel<NoopRawMutex, DisplayResponse, 1> =
            make_static!(Channel::new());
        #[cfg(feature = "radio")]
        let mqtt =
            crate::mqtt::MqttModule::new(spawner, &wifi.stack(), display_request, display_response)
                .await?;
        #[cfg(feature = "flash")]
        let state = flash.load().await?;
        #[cfg(not(feature = "flash"))]
        let state = AppSettings::default();
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
            #[cfg(feature = "display")]
            display,
            settings: RefCell::new(state.clone()),
            display_request,
            display_response,
        });
        Ok(application)
    }
    pub fn set_settings(&self, settings: &AppSettings) -> Result<(), WriteSettingsError> {
        let mut old = self.settings.borrow_mut();
        if old.wifi != settings.wifi {
            old.wifi = settings.wifi.clone();
            #[cfg(feature = "radio")]
            self.wifi.set_settings(settings.wifi.clone());
        }
        if old.mqtt != settings.mqtt {
            old.mqtt = settings.mqtt.clone();
            #[cfg(feature = "radio")]
            self.mqtt.set_settings(settings.mqtt.clone());
        }
        if old.display != settings.display {
            old.display = settings.display.clone();
            #[cfg(feature = "display")]
            self.display.set_settings(settings.display.clone());
        }
        #[cfg(feature = "flash")]
        self.flash.save(settings)?;
        Ok(())
    }

    fn initialize_settings(&'static self) -> Result<(), Error> {
        #[cfg(feature = "radio")]
        {
            let state = self.settings.borrow();

            self.wifi.set_settings(state.wifi.clone());
            self.mqtt.set_settings(state.mqtt.clone());
            #[cfg(feature = "display")]
            self.display.set_settings(state.display.clone());
        }
        Ok(())
    }
    fn spawn_tasks(&'static self) -> Result<(), Error> {
        #[cfg(feature = "radio")]
        self.ble.start(self)?;
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
        #[cfg(feature = "setup")]
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn handle_setup(application: &'static Application) {
                application
                    .handle_setup(
                        application.runtime.usb.usb_setup.requests(),
                        application.runtime.usb.usb_setup.responses(),
                        true,
                    )
                    .await;
            }
            handle_setup(self)?
        });
        #[cfg(all(feature = "setup", feature = "radio"))]
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn handle_setup(application: &'static Application) {
                application
                    .handle_setup(
                        application.ble.requests(),
                        application.ble.responses(),
                        false,
                    )
                    .await;
            }
            handle_setup(self)?
        });
        #[cfg(feature = "setup")]
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn update_mqtt_status(application: &'static Application) {
                if let Err(e) = application.update_mqtt_status().await {
                    error!("{:?}", e);
                }
            }
            update_mqtt_status(self)?
        });
        #[cfg(all(feature = "setup", feature = "radio"))]
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn update_wifi_status(application: &'static Application) {
                if let Err(e) = application.update_wifi_status().await {
                    error!("{:?}", e);
                }
            }
            update_wifi_status(self)?
        });
        Ok(())
    }
    async fn display_message(&'static self) {
        loop {
            let request = self.display_request.wait().await;
            match request {
                DisplayRequest::Run(msg) => {
                    let mut renderer =
                        glyph_render::Renderer::<MAX_GLYPHS>::new(glyph_list::LETTERS);
                    if let Err(e) = renderer.append(&msg) {
                        error!("{MODULE} error when rendering message: {:?}", e);
                    }
                    let glyphs = renderer.finish();
                    let glyph_strs: Vec<String<MAX_GLYPH_BYTES>, MAX_GLYPHS> = glyphs
                        .iter()
                        .map(|i| LETTERS[*i].try_into().unwrap_or(" ".try_into().unwrap()))
                        .collect();
                    self.display_response
                        .send(DisplayResponse::Start(glyph_strs.clone()))
                        .await;
                    #[cfg(not(feature = "display"))]
                    Timer::after_millis(1000).await;
                    #[cfg(feature = "display")]
                    {
                        info!("{MODULE} Displaying {}", msg);
                        // display.set_settings(self.state.borrow().display.clone());
                        if let Err(e) = self.display.run(&glyphs).await {
                            error!("{MODULE} error when displaying message: {:?}", e);
                        }
                    }
                    self.display_response
                        .send(DisplayResponse::Stop(glyph_strs))
                        .await;
                }
                DisplayRequest::Test => {
                    #[cfg(feature = "display")]
                    {
                        for index in (0..glyph_list::LETTERS.len()).step_by(3) {
                            let mut msg = Vec::<usize, MAX_GLYPHS>::new();
                            for _ in 0..MAX_GLYPHS {
                                msg.push(index).ok();
                            }
                            if let Err(e) = self.display.run(&msg).await {
                                error!("{MODULE} error when displaying message: {:?}", e);
                            }
                            Timer::after_millis(1000).await;
                        }
                    }
                }
                DisplayRequest::DeviceInfo => {
                    self.display_response
                        .send(DisplayResponse::DeviceInfo(self.device_info()))
                        .await;
                }
            }
        }
    }
    async fn handle_commands(&'static self) {
        let usb_serial = self.runtime.usb.usb_serial;
        loop {
            let command = usb_serial.commands().receive().await;
            let Ok(command) = str::from_utf8(&command) else {
                usb_serial
                    .write_feedback_line(format_args!("Command is not valid utf-8"))
                    .await;
                continue;
            };
            usb_serial
                .write_feedback_line(format_args!(">{}", command))
                .await;
            let command = Command::parse(command);
            let command = match command {
                Ok(command) => command,
                Err(e) => {
                    usb_serial
                        .write_feedback_line(format_args!("Bad command: {}", e))
                        .await;
                    continue;
                }
            };
            self.handle_command(command).await;
        }
    }
    async fn handle_command(&'static self, command: Command<'_>) {
        let usb_serial = self.runtime.usb.usb_serial;
        match command {
            Command::Help => {
                usb_serial
                    .write_feedback_line(format_args!("commands: help, display"))
                    .await;
            }
            Command::Display(msg) => {
                self.display_request
                    .signal(DisplayRequest::Run(msg.try_into().unwrap_or_default()));
            }
            Command::Test => {
                self.display_request.signal(DisplayRequest::Test);
            }
        }
    }
    #[cfg(feature = "setup")]
    async fn handle_setup(
        &'static self,
        requests: DynamicReceiver<'static, SetupRequest>,
        responses: DynamicSender<'static, SetupResponse>,
        secure: bool,
    ) {
        loop {
            let request = requests.receive().await;
            let response;
            match request {
                SetupRequest::ReadSettings => {
                    let mut settings = self.settings.borrow().clone();
                    if !secure {
                        settings.wifi.password.clear();
                        settings.mqtt.password.clear();
                    }
                    response = SetupResponse::ReadSettings(settings);
                }
                SetupRequest::WriteSettings(settings) => {
                    response = SetupResponse::WriteSettings(self.set_settings(&settings));
                }
                SetupRequest::TouchAppStatus => {
                    self.runtime.usb.usb_setup.update_status(|x| {});
                    self.ble.update_status(|x| {});
                    response = SetupResponse::TouchAppStatus;
                }
                SetupRequest::DeviceInfo => {
                    response = SetupResponse::DeviceInfo(self.device_info())
                }
            }
            responses.send(response).await;
        }
    }
    fn device_info(&self) -> DeviceInfo {
        DeviceInfo {
            serial: get_chipid().ok().unwrap_or(0),
            git_version: built_info::GIT_VERSION
                .unwrap_or("<unknown>")
                .try_into()
                .unwrap_or("<overflow>".try_into().unwrap()),
            git_dirty: built_info::GIT_DIRTY,
            git_head_ref: built_info::GIT_HEAD_REF
                .unwrap_or("<unknown>")
                .try_into()
                .unwrap_or("<overflow>".try_into().unwrap()),
            #[cfg(feature = "display")]
            glyphs: self.driver.count().unwrap_or(0),
            #[cfg(not(feature = "display"))]
            glyphs: 0,
            background: self.settings.borrow().display.background.clone(),
            foreground: self.settings.borrow().display.foreground.clone(),
        }
    }
    #[cfg(feature = "setup")]
    async fn update_mqtt_status(&self) -> Result<(), Error> {
        let mut mqtt_status = self.mqtt.watch_status().ok_or(Error::NotEnoughReceivers)?;
        loop {
            let mqtt_status = mqtt_status.changed().await;
            self.runtime.usb.usb_setup.update_status(|status| {
                status.mqtt_status = mqtt_status.clone();
            });
            #[cfg(feature = "radio")]
            self.ble.update_status(|status| {
                status.mqtt_status = mqtt_status.clone();
            });
        }
    }
    #[cfg(all(feature = "setup", feature = "radio"))]
    async fn update_wifi_status(&self) -> Result<(), Error> {
        let mut wifi_status = self.wifi.watch_status().ok_or(Error::NotEnoughReceivers)?;
        loop {
            let wifi_status = wifi_status.changed().await;
            self.runtime.usb.usb_setup.update_status(|status| {
                status.wifi_status = wifi_status.clone();
            });
            self.ble.update_status(|status| {
                status.wifi_status = wifi_status.clone();
            });
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
            "{MODULE} GIT_COMMIT_HASH: {}",
            built_info::GIT_COMMIT_HASH.unwrap_or("<unknown>")
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

use crate::cli::{Adjustment, Command, MqttField, WifiField};
use crate::error::Error;
use crate::peripherals::AppPeripherals;
use crate::product::{built_info, serial_number};
use crate::runtime::RuntimeModule;
use core::cell::RefCell;
use core::future::pending;
use core::mem;
use core::num::ParseIntError;
use embassy_executor::Spawner;
use embassy_rp::clocks::RoscRng;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use embassy_sync::watch::Watch;
use embassy_time::Timer;
use heapless::{CapacityError, String, Vec, format};
use letters::LETTERS;
use log::{error, info};
use proto::display::MAX_GLYPH_BYTES;
use proto::display::MAX_GLYPHS;
use proto::display::{DisplayRequest, DisplayResponse};
use proto::setup::{AppSettings, AppStatus, SetupRequest, SetupResponse, WriteSettingsError};
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
    display_request: &'static Signal<NoopRawMutex, DisplayRequest>,
    display_response: &'static Signal<NoopRawMutex, DisplayResponse>,
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
        let display_response: &'static Signal<NoopRawMutex, DisplayResponse> =
            make_static!(Signal::new());
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
        }
        #[cfg(feature = "flash")]
        self.flash.save(settings)?;
        Ok(())
    }

    fn initialize_settings(&'static self) -> Result<(), Error> {
        #[cfg(feature = "radio")]
        {
            let state = self.settings.borrow();
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
        // #[cfg(feature = "radio")]
        // self.spawner.spawn({
        //     #[embassy_executor::task]
        //     async fn notify_mqtt_status(application: &'static Application) {
        //         application.notify_mqtt_status().await;
        //     }
        //     notify_mqtt_status(self)?
        // });
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
                application.handle_setup().await;
            }
            handle_setup(self)?
        });
        #[cfg(feature = "radio")]
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn update_mqtt_status(application: &'static Application) {
                if let Err(e) = application.update_mqtt_status().await {
                    error!("{:?}", e);
                }
            }
            update_mqtt_status(self)?
        });
        #[cfg(feature = "radio")]
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
    // #[cfg(feature = "radio")]
    // async fn notify_mqtt_status(&'static self) {
    //     let Some(mut receiver) = self.mqtt.watch_status() else {
    //         error!("not enough receivers");
    //         return;
    //     };
    //     loop {
    //         let status = receiver.changed().await;
    //         self.ble
    //             .set_and_notify(
    //                 &self.ble.server().flappy_service.mqtt_status,
    //                 &format!("{}", status).unwrap_or_default(),
    //             )
    //             .await;
    //     }
    // }
    async fn display_message(&'static self) {
        #[cfg(feature = "display")]
        let mut display = crate::display::Display::new(self.driver);
        loop {
            let request = self.display_request.wait().await;
            match request {
                proto::display::DisplayRequest::Run(msg) => {
                    let mut renderer = render::Renderer::<MAX_GLYPHS>::new(letters::LETTERS);
                    if let Err(e) = renderer.append(&msg) {
                        error!("{MODULE} error when rendering message: {:?}", e);
                        continue;
                    }
                    let glyphs = renderer.finish();
                    let glyph_strs: Vec<String<MAX_GLYPH_BYTES>, MAX_GLYPHS> = glyphs
                        .iter()
                        .map(|i| LETTERS[*i].try_into().unwrap_or(" ".try_into().unwrap()))
                        .collect();
                    self.display_response
                        .signal(DisplayResponse::Start(glyph_strs.clone()));
                    #[cfg(not(feature = "display"))]
                    Timer::after_millis(1000).await;
                    #[cfg(feature = "display")]
                    {
                        info!("{MODULE} Displaying {}", msg);
                        // display.set_settings(self.state.borrow().display.clone());
                        if let Err(e) = display.run(&glyphs).await {
                            error!("{MODULE} error when displaying message: {:?}", e);
                        }
                    }
                    self.display_response
                        .signal(DisplayResponse::Stop(glyph_strs));
                }
                proto::display::DisplayRequest::Test => {
                    #[cfg(feature = "display")]
                    {
                        display.set_settings(self.settings.borrow().display.clone());

                        for index in (0..letters::LETTERS.len()).step_by(3) {
                            let mut msg = Vec::<usize, MAX_GLYPHS>::new();
                            for _ in 0..MAX_GLYPHS {
                                msg.push(index).ok();
                            }
                            if let Err(e) = display.run(&msg).await {
                                error!("{MODULE} error when displaying message: {:?}", e);
                            }
                            Timer::after_millis(1000).await;
                        }
                    }
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
            // Command::CalibrateRead => {
            //     let calibration = self.state.borrow().display.calibration.clone();
            //     usb_serial
            //         .write_feedback_line(format_args!("calibration = {:?}", calibration))
            //         .await;
            // }
            // Command::CalibrateReadOne(index) => {
            //     let calibration = self
            //         .state
            //         .borrow()
            //         .display
            //         .calibration
            //         .get(index)
            //         .cloned()
            //         .unwrap_or(0);
            //     usb_serial
            //         .write_feedback_line(format_args!("calibration = {}", calibration))
            //         .await;
            // }
            // Command::CalibrateWriteOne(index, adj, value) => {
            //     let mut state = self.state.borrow_mut();
            //     let calibration = &mut state.display.calibration;
            //     if calibration.len() < index + 1 {
            //         if let Err(_) = calibration.resize(index + 1, 0) {
            //             let cap = calibration.capacity();
            //             mem::drop(state);
            //             usb_serial
            //                 .write_feedback_line(format_args!(
            //                     "Index {} out of bounds {}",
            //                     index, cap
            //                 ))
            //                 .await;
            //             return;
            //         }
            //     }
            //     match adj {
            //         Adjustment::Add => calibration[index] += value,
            //         Adjustment::Sub => calibration[index] -= value,
            //         Adjustment::Set => calibration[index] = value,
            //     }
            //     let calibration = calibration[index];
            //     #[cfg(feature = "flash")]
            //     if let Err(e) = self.flash.save(&state) {
            //         error!("{MODULE} failed to update settings in flash {}", e);
            //     }
            //     mem::drop(state);
            //     usb_serial
            //         .write_feedback_line(format_args!("calibration = {}", calibration))
            //         .await;
            // }
            Command::Help => {
                usb_serial
                    .write_feedback_line(format_args!("commands: help, display"))
                    .await;
            }
            Command::Display(msg) => {
                self.display_request
                    .signal(DisplayRequest::Run(msg.try_into().unwrap_or_default()));
            }
            // Command::WifiRead => {
            //     let settings = self.state.borrow().wifi.clone();
            //     usb_serial
            //         .write_feedback_line(format_args!("WiFi settings: {:?}", settings))
            //         .await;
            // }
            // Command::WifiWrite(param, value) => {
            //     let mut state = self.state.borrow_mut();
            //     let settings = &mut state.wifi;
            //     if let Err::<(), Error>(e) = try {
            //         match param {
            //             WifiField::Ssid => {
            //                 settings.ssid = value.try_into()?;
            //             }
            //             WifiField::Password => {
            //                 settings.password = value.try_into()?;
            //             }
            //         }
            //     } {
            //         mem::drop(state);
            //         usb_serial
            //             .write_feedback_line(format_args!("Bad input: {}", e))
            //             .await;
            //         return;
            //     }
            //     #[cfg(feature = "flash")]
            //     if let Err(e) = self.flash.save(&state) {
            //         error!("{MODULE} failed to update settings in flash {}", e);
            //     }
            //     #[cfg(feature = "radio")]
            //     self.wifi.set_settings(state.wifi.clone());
            // }
            // Command::MqttRead => {
            //     let settings = self.state.borrow().mqtt.clone();
            //     usb_serial
            //         .write_feedback_line(format_args!("MQTT settings: {:?}", settings))
            //         .await;
            // }
            // Command::MqttWrite(param, value) => {
            //     let mut state = self.state.borrow_mut();
            //     let settings = &mut state.mqtt;
            //     if let Err::<(), Error>(e) = try {
            //         match param {
            //             MqttField::Hostname => {
            //                 settings.hostname = value.try_into()?;
            //             }
            //             MqttField::Port => {
            //                 settings.port = value.parse()?;
            //             }
            //             MqttField::Username => {
            //                 settings.username = value.try_into()?;
            //             }
            //             MqttField::Password => {
            //                 settings.password = value.try_into()?;
            //             }
            //             MqttField::Topic => {
            //                 settings.topic = value.try_into()?;
            //             }
            //         }
            //     } {
            //         mem::drop(state);
            //         usb_serial
            //             .write_feedback_line(format_args!("Bad input: {}", e))
            //             .await;
            //         return;
            //     }
            //     #[cfg(feature = "flash")]
            //     if let Err(e) = self.flash.save(&state) {
            //         error!("{MODULE} failed to update settings in flash {}", e);
            //     }
            //     #[cfg(feature = "radio")]
            //     self.mqtt.set_settings(state.mqtt.clone());
            // }
            Command::Test => {
                self.display_request.signal(DisplayRequest::Test);
            }
        }
    }
    #[cfg(feature = "setup")]
    async fn handle_setup(&'static self) {
        loop {
            let request = self.runtime.usb.usb_setup.receive_request().await;
            let response;
            match request {
                SetupRequest::ReadSettings => {
                    response = SetupResponse::ReadSettings(self.settings.borrow().clone());
                }
                SetupRequest::WriteSettings(settings) => {
                    response = SetupResponse::WriteSettings(self.set_settings(&settings));
                }
                SetupRequest::TouchAppStatus => {
                    self.runtime.usb.usb_setup.update_status(|x| {});
                    response = SetupResponse::TouchAppStatus;
                }
            }
            self.runtime.usb.usb_setup.send_response(&response).await;
        }
    }
    #[cfg(feature = "radio")]
    async fn update_mqtt_status(&self) -> Result<(), Error> {
        let mut mqtt_status = self.mqtt.watch_status().ok_or(Error::NotEnoughReceivers)?;
        loop {
            let mqtt_status = mqtt_status.changed().await;
            self.runtime.usb.usb_setup.update_status(|status| {
                status.mqtt_status = mqtt_status.clone();
            });
        }
    }
    #[cfg(feature = "radio")]
    async fn update_wifi_status(&self) -> Result<(), Error> {
        let mut mqtt_status = self.wifi.watch_status().ok_or(Error::NotEnoughReceivers)?;
        loop {
            let mqtt_status = mqtt_status.changed().await;
            self.runtime.usb.usb_setup.update_status(|status| {
                status.wifi_status = mqtt_status.clone();
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

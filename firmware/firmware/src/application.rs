use crate::cli::{Adjustment, Command, MqttField, TestType, WifiField};
use crate::error::Error;
use crate::peripherals::AppPeripherals;
use crate::product::{built_info, serial_number};
use crate::runtime::RuntimeModule;
use crate::{make_static, product};
use core::cell::RefCell;
use core::future::pending;
use core::mem;
use core::num::ParseIntError;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_rp::clocks::RoscRng;
use embassy_rp::otp::get_chipid;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::{Channel, DynamicReceiver, DynamicSender};
use embassy_sync::signal::Signal;
use embassy_sync::watch::Watch;
use embassy_time::Duration;
use embassy_time::Timer;
use heapless::{CapacityError, String, Vec, format};
use log::{error, info};
use protocol::display::MAX_GLYPH_BYTES;
use protocol::display::MAX_GLYPHS;
use protocol::display::{DisplayRequest, DisplayResponse};
use protocol::setup::FLAP_COUNT;
use protocol::setup::{
    AppSettings, AppStatus, DeviceInfo, SetupRequest, SetupResponse, WriteSettingsError,
};

pub enum DisplayResponseContainer {
    DisplayResponse(DisplayResponse),
    DeviceInfo(DeviceInfo),
}

pub const MODULE: &'static str = "[APP  ]";
pub struct Application {
    spawner: Spawner,
    runtime: &'static RuntimeModule,
    #[cfg(feature = "flash")]
    flash: &'static crate::flash::FlashModule,
    #[cfg(feature = "ble")]
    ble: &'static crate::ble::BleModule,
    #[cfg(feature = "radio")]
    led: &'static crate::led::LedModule,
    #[cfg(feature = "wifi")]
    wifi: &'static crate::wifi::WifiModule,
    #[cfg(feature = "mqtt")]
    mqtt: &'static crate::mqtt::MqttModule,
    #[cfg(feature = "display")]
    driver: &'static crate::driver::DriverModule,
    #[cfg(feature = "display")]
    controller: &'static crate::controller::ControllerModule,
    #[cfg(feature = "display")]
    display: &'static crate::display::DisplayModule,
    #[cfg(feature = "spindle")]
    spindle: &'static crate::spindle::SpindleModule,
    display_request: &'static Signal<NoopRawMutex, DisplayRequest>,
    display_response: &'static Channel<NoopRawMutex, DisplayResponseContainer, 1>,
    settings: RefCell<AppSettings>,
}

fn trim_null<const N: usize>(mut x: String<N>) -> String<N> {
    if x.as_bytes().last() == Some(&b'\0') {
        x.pop();
    }
    x
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
        let mut controller = crate::controller::ControllerModule::new(driver);
        let mut rng = RoscRng;
        #[cfg(feature = "flash")]
        let flash = crate::flash::FlashModule::new(peri.flash_peri).await?;
        #[cfg(feature = "radio")]
        let radio_drivers = crate::radio::RadioModule::new(spawner, peri.radio_peri).await?;
        #[cfg(feature = "radio")]
        let led = crate::led::LedModule::new(spawner, radio_drivers.module).await?;
        #[cfg(feature = "ble")]
        let ble = crate::ble::BleModule::new(spawner, radio_drivers.ble).await?;
        #[cfg(feature = "wifi")]
        let wifi = crate::wifi::WifiModule::new(
            spawner,
            radio_drivers.module,
            radio_drivers.net,
            &mut rng,
        )
        .await?;
        let display_request = make_static!(Signal<NoopRawMutex, DisplayRequest>, Signal::new());
        let display_response =
            make_static!(Channel<NoopRawMutex, DisplayResponseContainer, 1>, Channel::new());
        #[cfg(feature = "mqtt")]
        let mqtt =
            crate::mqtt::MqttModule::new(spawner, &wifi.stack(), display_request, display_response)
                .await?;
        #[cfg(feature = "flash")]
        let state = flash.load().await?;
        #[cfg(not(feature = "flash"))]
        let state = AppSettings::default();
        #[cfg(feature = "spindle")]
        let spindle = crate::spindle::SpindleModule::new();
        #[cfg(feature = "display")]
        let mut display = crate::display::DisplayModule::new(controller, display_response);
        let application = make_static!(
            Application,
            Application {
                spawner,
                runtime,
                #[cfg(feature = "flash")]
                flash,
                #[cfg(feature = "ble")]
                ble,
                #[cfg(feature = "wifi")]
                wifi,
                #[cfg(feature = "radio")]
                led,
                #[cfg(feature = "mqtt")]
                mqtt,
                #[cfg(feature = "display")]
                driver,
                #[cfg(feature = "display")]
                controller,
                #[cfg(feature = "display")]
                display,
                #[cfg(feature = "spindle")]
                spindle,
                settings: RefCell::new(state.clone()),
                display_request,
                display_response,
            }
        );
        Ok(application)
    }
    pub fn set_settings(&self, settings: &AppSettings) -> Result<(), WriteSettingsError> {
        let mut old = self.settings.borrow_mut();
        if old.wifi != settings.wifi {
            old.wifi = settings.wifi.clone();
            #[cfg(feature = "wifi")]
            self.wifi.set_settings(settings.wifi.clone());
        }
        if old.mqtt != settings.mqtt {
            old.mqtt = settings.mqtt.clone();
            #[cfg(feature = "mqtt")]
            self.mqtt.set_settings(settings.mqtt.clone());
        }
        if old.display != settings.display {
            old.display = settings.display.clone();
            #[cfg(feature = "display")]
            self.controller.set_settings(settings.display.clone());
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
            #[cfg(feature = "wifi")]
            self.wifi.set_settings(state.wifi.clone());
            #[cfg(feature = "mqtt")]
            self.mqtt.set_settings(state.mqtt.clone());
            #[cfg(feature = "display")]
            self.controller.set_settings(state.display.clone());
            #[cfg(feature = "display")]
            self.display.set_settings(state.display.clone());
        }
        Ok(())
    }
    fn spawn_tasks(&'static self) -> Result<(), Error> {
        #[cfg(feature = "ble")]
        self.ble.start()?;
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn display_message(application: &'static Application) {
                application.handle_requests().await;
            }
            display_message(self)?
        });
        #[cfg(feature = "usb")]
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn handle_commands(application: &'static Application) {
                application.handle_commands().await;
            }
            handle_commands(self)?
        });
        #[cfg(all(feature = "usb", feature = "setup"))]
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
        #[cfg(all(feature = "setup", feature = "radio", feature = "ble"))]
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
        #[cfg(all(feature = "setup", feature = "mqtt"))]
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn update_mqtt_status(application: &'static Application) {
                if let Err(e) = application.update_mqtt_status().await {
                    error!("{:?}", e);
                }
            }
            update_mqtt_status(self)?
        });
        #[cfg(all(feature = "setup", feature = "radio", feature = "wifi"))]
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn update_wifi_status(application: &'static Application) {
                if let Err(e) = application.update_wifi_status().await {
                    error!("{:?}", e);
                }
            }
            update_wifi_status(self)?
        });
        #[cfg(all(feature = "mqtt"))]
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn update_mqtt_device_info(application: &'static Application) {
                if let Err(e) = application.update_mqtt_device_info().await {
                    error!("{:?}", e);
                }
            }
            update_mqtt_device_info(self)?
        });
        Ok(())
    }

    async fn handle_requests(&'static self) {
        let mut request_state = None;
        loop {
            if let Some(r) = request_state.take() {
                match select(self.display_request.wait(), self.handle_request(r)).await {
                    Either::First(r) => {
                        request_state = Some(r);
                    }
                    Either::Second(()) => {}
                }
            } else {
                request_state = Some(self.display_request.wait().await);
            }
        }
    }
    async fn handle_request(&'static self, request: DisplayRequest) {
        match request {
            DisplayRequest::Run(msg) => {
                self.display.display_once(&msg).await;
            }
            DisplayRequest::Test => {
                #[cfg(feature = "display")]
                {
                    for index in (0..FLAP_COUNT).step_by(3) {
                        let mut msg = Vec::<usize, MAX_GLYPHS>::new();
                        for _ in 0..MAX_GLYPHS {
                            msg.push(index).ok();
                        }
                        if let Err(e) = self.controller.run(&msg).await {
                            error!("{MODULE} error when displaying message: {:?}", e);
                        }
                        Timer::after_millis(1000).await;
                    }
                }
            }
            DisplayRequest::RunSpindle(src) => {
                #[cfg(feature = "spindle")]
                {
                    self.spindle.run_program(&src, self.display).await;
                }
            }
        }
    }

    #[cfg(feature = "usb")]
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
    #[cfg(feature = "usb")]
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
            Command::Test(typ) => match typ {
                TestType::Spin => {
                    self.display_request.signal(DisplayRequest::Test);
                }
                TestType::Read => {
                    #[cfg(feature = "display")]
                    self.driver.run_read_test().await;
                }
            },
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
                    #[cfg(all(feature = "usb", feature = "setup"))]
                    self.runtime.usb.usb_setup.update_status(|x| {});
                    #[cfg(feature = "ble")]
                    self.ble.update_status(|x| {});
                    response = SetupResponse::TouchAppStatus;
                }
                SetupRequest::DeviceInfo => {
                    response = SetupResponse::DeviceInfo(self.device_info())
                }
                SetupRequest::Ping => {
                    response = SetupResponse::Pong;
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
    #[cfg(all(feature = "setup", feature = "mqtt"))]
    async fn update_mqtt_status(&self) -> Result<(), Error> {
        let mut mqtt_status = self.mqtt.watch_status().ok_or(Error::NotEnoughReceivers)?;
        loop {
            let mqtt_status = mqtt_status.changed().await;
            self.runtime.usb.usb_setup.update_status(|status| {
                status.mqtt_status = mqtt_status.clone();
            });
            #[cfg(feature = "ble")]
            self.ble.update_status(|status| {
                status.mqtt_status = mqtt_status.clone();
            });
        }
    }
    #[cfg(all(feature = "setup", feature = "radio", feature = "wifi"))]
    async fn update_wifi_status(&self) -> Result<(), Error> {
        let mut wifi_status = self.wifi.watch_status().ok_or(Error::NotEnoughReceivers)?;
        loop {
            let wifi_status = wifi_status.changed().await;
            self.runtime.usb.usb_setup.update_status(|status| {
                status.wifi_status = wifi_status.clone();
            });
            #[cfg(feature = "ble")]
            self.ble.update_status(|status| {
                status.wifi_status = wifi_status.clone();
            });
        }
    }
    #[cfg(all(feature = "mqtt"))]
    async fn update_mqtt_device_info(&self) -> Result<(), Error> {
        let mut info = self.device_info();
        loop {
            self.display_response
                .send(DisplayResponseContainer::DeviceInfo(info.clone()))
                .await;
            loop {
                Timer::after(Duration::from_secs(1)).await;
                let new = self.device_info();
                if info != new {
                    info = new;
                    break;
                }
            }
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
        pending::<!>().await
    } {
        error!("{MODULE} Uncaught error: {:?}", e);
    }
}

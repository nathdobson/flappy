use crate::bootsel::BootselModule;
use crate::cli::{Adjustment, Command, MqttField, TestType, WifiField};
use crate::error::Error;
use crate::kernel::KernelModule;
use crate::peripherals::AppPeripherals;
use crate::{Irqs, built_info};
use board_info::serial_number;
use core::cell::RefCell;
use core::future::pending;
use core::mem;
use core::num::ParseIntError;
use embassy_executor::Spawner;
use embassy_executor::raw::TaskPool;
use embassy_futures::select::{Either, select};
use embassy_rp::clocks::RoscRng;
use embassy_rp::otp::get_chipid;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::{Channel, DynamicReceiver, DynamicSender};
use embassy_sync::signal::Signal;
use embassy_sync::watch::Watch;
use embassy_time::Timer;
use embassy_time::{Delay, Duration};
use embedded_hal_async::delay::DelayNs;
use heapless::{CapacityError, String, Vec, format};
use log::{error, info};
use make_static::make_static;
use protocol::display::MAX_GLYPH_BYTES;
use protocol::display::MAX_GLYPHS;
use protocol::display::{DisplayRequest, DisplayResponse};
use protocol::setup::{
    AppSettings, AppStatus, DeviceInfo, SetupRequest, SetupResponse, WriteSettingsError,
};
use protocol::setup::{FLAP_COUNT, WriteAppSettings};
#[cfg(feature = "radio")]
use radio_builder::RadioBuilder;
#[cfg(feature = "wifi")]
use radio_builder::wifi::{WifiBuilder, WifiStack};
use runtime::LocalSpawn;
// No more than half of stack space should be used when serializing/deserializing.
const _: [u8; 1] = [0; (size_of::<SetupRequest>() < 2048) as usize];
const _: [u8; 1] = [0; (size_of::<SetupResponse>() < 2048) as usize];
const WIFI_SOCKETS: usize = 5;

pub const MODULE: &'static str = "[APP  ]";
pub struct Application {
    spawner: Spawner,
    runtime: &'static KernelModule,
    #[cfg(feature = "usb")]
    usb: &'static crate::usb::UsbModule,
    #[cfg(feature = "flash")]
    flash: &'static crate::flash::FlashModule,
    #[cfg(feature = "ble")]
    ble: &'static crate::ble::BleModule,
    #[cfg(feature = "radio")]
    led: &'static crate::blink::BlinkModule,
    #[cfg(feature = "wifi")]
    wifi: &'static radio_builder::wifi::Wifi,
    #[cfg(feature = "mqtt")]
    mqtt: &'static crate::mqtt::MqttModule,
    #[cfg(feature = "display")]
    driver: &'static crate::driver::DriverModule,
    #[cfg(feature = "display")]
    controller: &'static crate::controller::ControllerModule,
    display: &'static crate::display::DisplayModule,
    #[cfg(feature = "spindle")]
    spindle: &'static crate::spindle::SpindleModule,
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
        runtime: &'static KernelModule,
        peri: AppPeripherals,
    ) -> Result<&'static Self, Error> {
        #[cfg(feature = "usb")]
        let usb = crate::usb::UsbModule::new(spawner, runtime.usb);
        let bootsel = BootselModule::new(spawner, peri.bootsel)?;
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
        let radio = RadioBuilder {
            spawner,
            peripherals: peri.radio_peri,
            pio_irq: Irqs,
            dma_irq: Irqs,
        }
        .build()
        .await?;
        // let radio_drivers = crate::radio::RadioModule::new(spawner, peri.radio_peri).await?;
        #[cfg(feature = "radio")]
        let led = crate::blink::BlinkModule::new(spawner, radio.led)?;
        #[cfg(feature = "ble")]
        let ble = crate::ble::BleModule::new(spawner, radio.ble, bootsel)?;
        #[cfg(feature = "wifi")]
        let wifi = WifiBuilder {
            spawner,
            peripherals: radio.wifi,
            stack: make_static!(WifiStack::<WIFI_SOCKETS>, WifiStack::new()),
        }
        .build()?;
        #[cfg(feature = "mqtt")]
        let mqtt = crate::mqtt::MqttModule::new(spawner, &wifi.stack())?;
        #[cfg(feature = "flash")]
        let state = flash.load().await?;
        #[cfg(not(feature = "flash"))]
        let state = AppSettings::default();
        #[cfg(feature = "spindle")]
        let spindle = crate::spindle::SpindleModule::new();
        let mut display = crate::display::DisplayModule::new(
            #[cfg(feature = "display")]
            controller,
            #[cfg(feature = "mqtt")]
            mqtt,
        );
        let application = make_static!(
            Application,
            Application {
                spawner,
                runtime,
                #[cfg(feature = "usb")]
                usb,
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
                display,
                #[cfg(feature = "spindle")]
                spindle,
                settings: RefCell::new(state.clone()),
            }
        );
        Ok(application)
    }
    pub fn set_settings(&self, settings: &WriteAppSettings) -> Result<(), WriteSettingsError> {
        let mut old = self.settings.borrow_mut();
        if let Some(wifi) = &settings.wifi {
            if old.wifi != *wifi {
                old.wifi = wifi.clone();
                #[cfg(feature = "wifi")]
                self.wifi.set_settings(wifi.clone());
            }
        }
        if let Some(mqtt) = &settings.mqtt {
            if old.mqtt != *mqtt {
                old.mqtt = mqtt.clone();
                #[cfg(feature = "mqtt")]
                self.mqtt.set_settings(mqtt.clone());
            }
        }
        if let Some(display) = &settings.display {
            if old.display != *display {
                old.display = display.clone();
                #[cfg(feature = "display")]
                self.controller.set_settings(display.clone());
                self.display.set_settings(display.clone());
            }
        }
        #[cfg(feature = "flash")]
        self.flash.save(&old)?;
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
            self.display.set_settings(state.display.clone());
        }
        Ok(())
    }
    fn spawn_tasks(&'static self) -> Result<(), Error> {
        #[cfg(feature = "mqtt")]
        make_static!(_, LocalSpawn::new(self.spawner)).spawn(|| async move {
            self.handle_requests().await;
        });

        #[cfg(feature = "usb")]
        make_static!(_, LocalSpawn::new(self.spawner)).spawn(|| async move {
            self.handle_commands().await;
        });

        #[cfg(all(feature = "usb", feature = "setup"))]
        make_static!(_, LocalSpawn::new(self.spawner)).spawn(|| async move {
            loop {
                let request = self.usb.receive_request().await;
                let response = self.handle_setup(&request, true).await;
                self.usb.send_response(&response).await;
            }
        });

        #[cfg(all(feature = "setup", feature = "radio", feature = "ble"))]
        make_static!(_, LocalSpawn::new(self.spawner)).spawn(|| async move {
            if let Err(e) = self
                .ble
                .advertise(async |request| {
                    self.handle_setup(request, false).await

                })
                .await
            {
                error!("BLE error {}", e);
            }
        });

        #[cfg(all(feature = "setup", feature = "mqtt"))]
        make_static!(_, LocalSpawn::new(self.spawner)).spawn(|| async move {
            if let Err(e) = self.update_mqtt_status().await {
                error!("{:?}", e);
            }
        });

        #[cfg(all(feature = "setup", feature = "radio", feature = "wifi"))]
        make_static!(_, LocalSpawn::new(self.spawner)).spawn(|| async move {
            if let Err(e) = self.update_wifi_status().await {
                error!("{:?}", e);
            }
        });

        #[cfg(all(feature = "mqtt"))]
        make_static!(_, LocalSpawn::new(self.spawner)).spawn(|| async move {
            if let Err(e) = self.update_mqtt_device_info().await {
                error!("{:?}", e);
            }
        });
        Ok(())
    }
    #[cfg(feature = "mqtt")]
    async fn handle_requests(&'static self) {
        let mut request_state = None;
        loop {
            if let Some(r) = request_state.take() {
                match select(self.mqtt.receive_request(), self.handle_request(r)).await {
                    Either::First(r) => {
                        request_state = Some(r);
                    }
                    Either::Second(()) => {}
                }
            } else {
                request_state = Some(self.mqtt.receive_request().await);
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
                    for index in (0..FLAP_COUNT).step_by(1) {
                        let mut msg = Vec::<usize, MAX_GLYPHS>::new();
                        for _ in 0..MAX_GLYPHS {
                            msg.push(index).ok();
                        }
                        if let Err(e) = self.controller.run(&msg).await {
                            error!("{MODULE} error when displaying message: {:?}", e);
                        }
                        Timer::after_millis(250).await;
                    }
                }
            }
            DisplayRequest::RunSpindle(src) => {
                #[cfg(feature = "spindle")]
                {
                    self.spindle
                        .run_program(
                            &src,
                            #[cfg(feature = "display")]
                            self.display,
                        )
                        .await;
                }
            }
        }
    }

    #[cfg(feature = "usb")]
    async fn handle_commands(&'static self) {
        let usb_terminal = self.usb.terminal();
        loop {
            let command = usb_terminal.commands().receive().await;
            let Ok(command) = str::from_utf8(&command) else {
                usb_terminal
                    .write_feedback_line(format_args!("Command is not valid utf-8"))
                    .await;
                continue;
            };
            usb_terminal
                .write_feedback_line(format_args!(">{}", command))
                .await;
            let command = Command::parse(command);
            let command = match command {
                Ok(command) => command,
                Err(e) => {
                    usb_terminal
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
        let usb_terminal = self.usb.terminal();
        match command {
            Command::Help => {
                usb_terminal
                    .write_feedback_line(format_args!("commands: help, display"))
                    .await;
            }
            Command::Test(typ) => match typ {
                TestType::Enable => {
                    #[cfg(feature = "display")]
                    self.driver.set_enabled(true);
                    Delay.delay_ms(100_000).await;
                    #[cfg(feature = "display")]
                    self.driver.set_enabled(false);
                }
                TestType::Read => {
                    #[cfg(feature = "display")]
                    self.driver.run_read_test().await;
                }
            },
        }
    }
    #[cfg(feature = "setup")]
    async fn handle_setup(&'static self, request: &SetupRequest, secure: bool) -> SetupResponse {
        match request {
            SetupRequest::ReadSettings => {
                let mut settings = self.settings.borrow().clone();
                if !secure {
                    settings.wifi.password.clear();
                    settings.mqtt.password.clear();
                }
                SetupResponse::ReadSettings(settings)
            }
            SetupRequest::WriteSettings(settings) => {
                SetupResponse::WriteSettings(self.set_settings(&settings))
            }
            SetupRequest::TouchAppStatus => {
                #[cfg(all(feature = "usb", feature = "setup"))]
                self.usb.update_status(|x| {});
                #[cfg(feature = "ble")]
                self.ble.update_status(|x| {});
                SetupResponse::TouchAppStatus
            }
            SetupRequest::DeviceInfo => SetupResponse::DeviceInfo(self.device_info()),
            SetupRequest::Ping => SetupResponse::Pong,
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
            #[cfg(feature = "usb")]
            self.usb.update_status(|status| {
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
            #[cfg(feature = "usb")]
            self.usb.update_status(|status| {
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
            self.mqtt.send_device_info(info.clone()).await;
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
pub async fn main_task(spawner: Spawner, runtime: &'static KernelModule, peri: AppPeripherals) {
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

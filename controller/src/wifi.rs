use crate::terminate::check_terminate;
use alloc::ffi::CString;
use arduino_core::delay::{delay, micros, millis};
use arduino_core::sprintln;
use arduino_wifi::wifi::{WlStatus, wifi_begin_wpa, wifi_disconnect, wifi_local_ip, wifi_status};
use core::ffi::CStr;
use core::fmt::{Display, Formatter};

#[derive(Eq, Ord, PartialEq, PartialOrd, Debug)]
enum State {
    Disconnected,
    Connecting(u32),
    WaitingForIp(u32),
    Connected,
}
pub struct WiFiConnector {
    ssid: CString,
    pass: CString,
    state: State,
}

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Debug)]
pub enum WiFiStatus {
    Unconfigured,
    Disconnected,
    Connecting,
    WaitingForIp,
    Connected,
}

impl WiFiStatus {
    pub fn as_c_str(&self) -> &CStr {
        match self {
            WiFiStatus::Unconfigured => c"Unconfigured",
            WiFiStatus::Disconnected => c"Disconnected",
            WiFiStatus::Connecting => c"Connecting",
            WiFiStatus::WaitingForIp => c"Waiting for IP address",
            WiFiStatus::Connected => c"Connected",
        }
    }
}

impl WiFiConnector {
    pub fn new(ssid: CString, pass: CString) -> Self {
        WiFiConnector {
            ssid,
            pass,
            state: State::Disconnected,
        }
    }
    pub fn status(&self) -> WiFiStatus {
        match self.state {
            State::Disconnected => WiFiStatus::Disconnected,
            State::Connecting(_) => WiFiStatus::Connecting,
            State::WaitingForIp(_) => WiFiStatus::WaitingForIp,
            State::Connected => WiFiStatus::Connected,
        }
    }
    pub fn step(&mut self) {
        match self.state {
            State::Disconnected => {
                sprintln!("Connecting to WiFi ssid {}...", self.ssid.to_str().unwrap());
                wifi_begin_wpa(&self.ssid, &self.pass);
                self.state = State::Connecting(millis());
            }
            State::Connecting(start) => {
                let status = wifi_status();
                if status == WlStatus::WlConnected {
                    sprintln!("Waiting for IP...");
                    self.state = State::WaitingForIp(start);
                } else if millis().wrapping_sub(start) > 10_000 {
                    sprintln!("Failed to connect to WiFi: '{:?}'.", status);
                    self.state = State::Disconnected;
                }
            }
            State::WaitingForIp(start) => {
                let ip = wifi_local_ip();
                if ip.is_none() || ip.unwrap().is_unspecified() {
                    sprintln!("Connected to WiFi.");
                    delay(1000);
                    self.state = State::Connected;
                } else if millis().wrapping_sub(start) > 10_000 {
                    sprintln!("Failed to get IP address.");
                    self.state = State::Disconnected;
                }
            }
            State::Connected => {
                //
            }
        }
    }
    pub fn connected(&self) -> bool {
        self.state == State::Connected
    }
}

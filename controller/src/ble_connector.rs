use crate::eeprom::EepromState;
use crate::irc::IrcStatus;
use crate::wifi::WiFiStatus;
use alloc::ffi::CString;
use alloc::vec::Vec;
use alloc::{format, vec};
use arduino_ble::ble_descriptor::BLEDescriptor;
use arduino_ble::ble_device::BLEDevice;
use arduino_ble::ble_int_characteristic::BLEIntCharacteristic;
use arduino_ble::ble_service::BLEService;
use arduino_ble::ble_string_characteristic::BLEStringCharacteristic;
use arduino_ble::permissions::{BLE_INDICATE, BLE_NOTIFY, BLE_READ, BLE_WRITE};
use arduino_ble::{
    ble_add_service, ble_advertise, ble_begin, ble_central, ble_set_advertised_service,
    ble_set_local_name,
};
use arduino_core::delay::delay;
use arduino_core::sprintln;
use core::ffi::CStr;
use core::mem;

const SERVICE_UUID: &'static CStr = c"5af0b930-b9b5-11f0-b558-0800200c9a66";
const WIFI_SSID_UUID: &'static CStr = c"71fe3670-b9b5-11f0-b558-0800200c9a66";
const WIFI_PASSWORD_UUID: &'static CStr = c"62c24b3b-fe0c-47c4-85cc-18eb853a8f43";
const WIFI_STATUS_UUID: &'static CStr = c"b63a97a1-ee72-4f44-9f82-c04c95c7d76e";
const IRC_HOSTNAME_UUID: &'static CStr = c"e789ca87-4fe2-4967-94ed-68f9d6cec087";
const IRC_PORT_UUID: &'static CStr = c"2b02c87e-260f-47d2-8fe0-83c87979fd01";
const IRC_NICKNAME_UUID: &'static CStr = c"7a9cfbb2-78fa-4222-be11-ab7d401d6f08";
const IRC_CHANNEL_UUID: &'static CStr = c"a2e39581-34e8-4613-b993-a50a32820841";
const IRC_STATUS_UUID: &'static CStr = c"ffa3be06-af36-4896-b336-02124a4dc539";

pub struct BLEConnector {
    service: BLEService,
    wifi_ssid: BLEStringCharacteristic,
    wifi_ssid_descriptor: BLEDescriptor,
    wifi_password: BLEStringCharacteristic,
    wifi_password_descriptor: BLEDescriptor,
    wifi_status: BLEStringCharacteristic,
    wifi_status_descriptor: BLEDescriptor,
    wifi_status_value: Option<WiFiStatus>,
    wifi_changed: bool,

    irc_hostname: BLEStringCharacteristic,
    irc_hostname_descriptor: BLEDescriptor,
    irc_port: BLEStringCharacteristic,
    irc_port_descriptor: BLEDescriptor,
    irc_nickname: BLEStringCharacteristic,
    irc_nickname_descriptor: BLEDescriptor,
    irc_channel: BLEStringCharacteristic,
    irc_channel_descriptor: BLEDescriptor,
    irc_status: BLEStringCharacteristic,
    irc_status_descriptor: BLEDescriptor,
    irc_status_value: Option<IrcStatus>,
    irc_changed: bool,

    device: Option<BLEDevice>,
}

const CHARACTERISTIC_NAME: &'static CStr = c"2901";

impl BLEConnector {
    pub fn set_wifi_status(&mut self, wifi_status: WiFiStatus) {
        if self.wifi_status_value != Some(wifi_status) {
            self.wifi_status_value = Some(wifi_status);
            self.wifi_status.write_value(wifi_status.as_c_str());
        }
    }
    pub fn set_irc_status(&mut self, irc_status: IrcStatus) {
        if self.irc_status_value != Some(irc_status) {
            self.irc_status_value = Some(irc_status);
            self.irc_status.write_value(irc_status.as_c_str());
        }
    }
    pub fn step(&mut self) {
        if self.device.is_none() {
            self.device = ble_central();
            if let Some(device) = &mut self.device {
                sprintln!("BLE connected");
            }
        }
        let Some(device) = &mut self.device else {
            return;
        };
        if !device.connected() {
            sprintln!("BLE disconnected");
            self.device = None;
        }
        if self.wifi_ssid.written() {
            sprintln!("Updated WiFi SSID {:?}", self.wifi_ssid.value());
        }
        if self.wifi_password.written() {
            sprintln!("Updated WiFi password");
            self.wifi_changed = true;
        }
        if self.irc_hostname.written() {
            sprintln!("Updated IRC hostname {:?}", self.irc_hostname.value());
        }
        if self.irc_port.written() {
            sprintln!("Updated IRC port {:?}", self.irc_port.value());
        }
        if self.irc_nickname.written() {
            sprintln!("Updated IRC nickname {:?}", self.irc_nickname.value());
        }
        if self.irc_channel.written() {
            sprintln!("Updated IRC channel {:?}", self.irc_channel.value());
            self.irc_changed = true;
        }
    }
    pub fn wifi_changed(&mut self) -> bool {
        mem::replace(&mut self.wifi_changed, false)
    }
    pub fn irc_changed(&mut self) -> bool {
        mem::replace(&mut self.irc_changed, false)
    }
}

impl BLEConnector {
    pub fn new(original: &EepromState) -> Self {
        let mut service = BLEService::new(SERVICE_UUID);
        sprintln!("Starting bluetooth");
        assert!(ble_begin());
        ble_set_local_name(c"Split Flap Display");

        let mut wifi_ssid = BLEStringCharacteristic::new(WIFI_SSID_UUID, BLE_READ | BLE_WRITE, 32);
        wifi_ssid.write_value(&original.wifi_ssid);
        let wifi_ssid_descriptor = BLEDescriptor::new(CHARACTERISTIC_NAME, b"WiFi SSID");
        wifi_ssid.add_descriptor(&wifi_ssid_descriptor);
        service.add_characteristic(&wifi_ssid);

        let mut wifi_password = BLEStringCharacteristic::new(WIFI_PASSWORD_UUID, BLE_WRITE, 63);
        wifi_password.write_value(&original.wifi_password);
        let wifi_password_descriptor = BLEDescriptor::new(CHARACTERISTIC_NAME, b"WiFi Password");
        wifi_password.add_descriptor(&wifi_password_descriptor);
        service.add_characteristic(&wifi_password);

        let mut wifi_status = BLEStringCharacteristic::new(
            WIFI_STATUS_UUID,
            BLE_READ | BLE_NOTIFY,
            32,
        );
        let wifi_status_descriptor = BLEDescriptor::new(CHARACTERISTIC_NAME, b"WiFi Status");
        wifi_status.add_descriptor(&wifi_status_descriptor);
        service.add_characteristic(&wifi_status);

        let mut irc_hostname =
            BLEStringCharacteristic::new(IRC_HOSTNAME_UUID, BLE_READ | BLE_WRITE, 255);
        irc_hostname.write_value(&original.irc_hostname);
        let irc_hostname_descriptor = BLEDescriptor::new(CHARACTERISTIC_NAME, b"IRC Hostname");
        irc_hostname.add_descriptor(&irc_hostname_descriptor);
        service.add_characteristic(&irc_hostname);

        let mut irc_port = BLEStringCharacteristic::new(IRC_PORT_UUID, BLE_READ | BLE_WRITE, 6);
        let mut irc_port_bytes: Vec<u8> = format!("{}", original.irc_port).into_bytes();
        irc_port_bytes.push(0);
        irc_port.write_value(&CString::from_vec_with_nul(irc_port_bytes).unwrap());
        let irc_port_descriptor = BLEDescriptor::new(CHARACTERISTIC_NAME, b"IRC Port");
        irc_port.add_descriptor(&irc_port_descriptor);
        service.add_characteristic(&irc_port);

        let mut irc_nickname =
            BLEStringCharacteristic::new(IRC_NICKNAME_UUID, BLE_READ | BLE_WRITE, 128);
        irc_nickname.write_value(&original.irc_nickname);
        let irc_nickname_descriptor = BLEDescriptor::new(CHARACTERISTIC_NAME, b"IRC Nickname");
        irc_nickname.add_descriptor(&irc_nickname_descriptor);
        service.add_characteristic(&irc_nickname);

        let mut irc_channel =
            BLEStringCharacteristic::new(IRC_CHANNEL_UUID, BLE_READ | BLE_WRITE, 128);
        irc_channel.write_value(&original.irc_channel);
        let irc_channel_descriptor = BLEDescriptor::new(CHARACTERISTIC_NAME, b"IRC Channel");
        irc_channel.add_descriptor(&irc_channel_descriptor);
        service.add_characteristic(&irc_channel);

        let mut irc_status =
            BLEStringCharacteristic::new(IRC_STATUS_UUID, BLE_READ | BLE_NOTIFY, 32);
        let irc_status_descriptor = BLEDescriptor::new(CHARACTERISTIC_NAME, b"IRC Status");
        irc_status.add_descriptor(&irc_status_descriptor);
        service.add_characteristic(&irc_status);

        ble_set_advertised_service(&service);
        ble_add_service(&service);
        ble_advertise();
        BLEConnector {
            service,
            wifi_ssid,
            wifi_ssid_descriptor,
            wifi_password,
            wifi_password_descriptor,
            wifi_status,
            wifi_status_descriptor,
            wifi_status_value: None,

            wifi_changed: false,
            irc_hostname,
            irc_hostname_descriptor,
            irc_port,
            irc_port_descriptor,
            irc_nickname,
            irc_nickname_descriptor,
            irc_channel,
            irc_channel_descriptor,
            irc_status,
            irc_status_descriptor,
            irc_status_value: None,

            irc_changed: false,
            device: None,
        }
    }
    pub fn wifi_ssid(&self) -> CString {
        self.wifi_ssid.value()
    }
    pub fn wifi_password(&self) -> CString {
        self.wifi_password.value()
    }
    pub fn irc_hostname(&self) -> CString {
        self.irc_hostname.value()
    }
    pub fn irc_port(&self) -> u16 {
        self.irc_port
            .value()
            .to_str()
            .unwrap_or("")
            .parse()
            .unwrap_or(0)
    }
    pub fn irc_nickname(&self) -> CString {
        self.irc_nickname.value()
    }
    pub fn irc_channel(&self) -> CString {
        self.irc_channel.value()
    }
}

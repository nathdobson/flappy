use alloc::ffi::CString;
use alloc::vec;
use alloc::vec::Vec;
use arduino_core::sprintln;
use arduino_eeprom::{eeprom_read, eeprom_write};
use postcard::ser_flavors::{AllocVec, Flavor};
use postcard::{from_bytes, serialize_with_flavor};
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct EepromState {
    pub wifi_ssid: CString,
    pub wifi_password: CString,
    pub irc_hostname: CString,
    pub irc_port: u16,
    pub irc_nickname: CString,
    pub irc_channel: CString,
}

impl EepromState {
    pub fn new() -> Self {
        EepromState {
            wifi_ssid: Default::default(),
            wifi_password: Default::default(),
            irc_hostname: Default::default(),
            irc_port: 0,
            irc_nickname: Default::default(),
            irc_channel: Default::default(),
        }
    }
    pub fn load() -> Self {
        let mut len = [0u8; 4];
        eeprom_read(0, &mut len);
        let len = u32::from_le_bytes(len) as usize;
        if len == 0 || len >= 1024 {
            return EepromState::new();
        }
        let mut buf = vec![0; len];
        eeprom_read(4, &mut buf);
        from_bytes(&buf).unwrap_or_else(|e| {
            sprintln!("Error deserializing eeprom: {:?}", e);
            EepromState::new()
        })
    }
    pub fn save(&self) {
        let mut v = AllocVec::new();
        v.try_extend(&[0u8; 4]).unwrap();
        let mut v = serialize_with_flavor(self, v).unwrap();
        *<&mut [u8; 4]>::try_from(&mut v[0..4]).unwrap() = ((v.len() - 4) as u32).to_le_bytes();
        eeprom_write(0, &v);
    }
    pub fn print_debug(&self) {
        sprintln!("EEPROM state:");
        sprintln!("WiFi SSID: {}", self.wifi_ssid.to_str().unwrap());
        sprintln!("WiFi pass: {}", !self.wifi_password.is_empty());
        sprintln!("IRC host: {}", self.irc_hostname.to_str().unwrap());
        sprintln!("IRC port: {}", self.irc_port);
        sprintln!("IRC nickname: {}", self.irc_nickname.to_str().unwrap());
        sprintln!("IRC channel: {}", self.irc_channel.to_str().unwrap());
    }
}

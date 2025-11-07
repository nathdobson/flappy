use crate::terminate::check_terminate;
use alloc::ffi::CString;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{format, vec};
use arduino_core::delay::{delay, millis};
use arduino_core::random::random;
use arduino_core::sprintln;
use arduino_wifi::wifi_ssl_client::WiFiSSLClient;
use core::ffi::CStr;
use core::fmt::{Display, Formatter};

pub struct IrcClient {
    client: WiFiSSLClient,
    backoff: Option<u32>,
    buffer: Vec<u8>,
    welcomed: bool,

    host: CString,
    port: u16,
    nick: CString,
    channel: CString,

    next_message: Option<String>,
}

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Debug)]
pub enum IrcStatus {
    Unconfigured,
    Disconnected,
    Connecting,
    Connected,
}

impl IrcStatus {
    pub fn as_c_str(&self) -> &CStr {
        match self {
            IrcStatus::Unconfigured => c"Unconfigured",
            IrcStatus::Disconnected => c"Disconnected",
            IrcStatus::Connecting => c"Connecting",
            IrcStatus::Connected => c"Connected",
        }
    }
}

impl IrcClient {
    pub fn new(host: CString, port: u16, nick: CString, channel: CString) -> IrcClient {
        IrcClient {
            client: WiFiSSLClient::new(),
            backoff: None,
            buffer: vec![],
            welcomed: false,
            host,
            port,
            nick,
            channel,
            next_message: None,
        }
    }
    pub fn status(&self) -> IrcStatus {
        if self.client.connected() {
            if self.welcomed {
                IrcStatus::Connected
            } else {
                IrcStatus::Connecting
            }
        } else {
            IrcStatus::Disconnected
        }
    }
    pub fn step(&mut self) {
        if let Some(backoff) = self.backoff {
            if millis().wrapping_sub(backoff) > 20_000 {
                self.backoff = None;
            } else {
                return;
            }
        }
        if !self.client.connected() {
            self.buffer.clear();
            self.welcomed = false;
            sprintln!(
                "Connecting to IRC host {} port {}...",
                self.host.to_str().unwrap(),
                self.port
            );
            if self.client.connect(&self.host, self.port) {
                assert!(self.client.connected());
                sprintln!("Connected to IRC.");
                let nick = format!("{}{}", self.nick.to_str().unwrap(), random(0, 100));
                let header = format!(
                    "PASS none\nNICK {}\nUSER {} 0 * : {}\nJOIN {}\n",
                    nick,
                    self.nick.to_str().unwrap(),
                    self.nick.to_str().unwrap(),
                    self.channel.to_str().unwrap()
                );
                assert_eq!(header.len(), self.client.write(header.as_bytes()));
            } else {
                self.backoff = Some(millis());
                sprintln!("Failed to connect.");
                return;
            }
        }
        let mut tmp = [0u8];
        while self.buffer.last() != Some(&b'\n') {
            if self.client.read(&mut tmp) == 0 {
                return;
            }
            if self.buffer.len() < 128 {
                self.buffer.push(tmp[0]);
            } else if tmp[0] == b'\n' {
                self.buffer.clear();
            }
        }
        self.buffer.pop();
        let line = str::from_utf8(&self.buffer).unwrap();
        sprintln!("Received: {}", line);
        if let Some((first, line)) = line.split_once(' ') {
            if first == "PING" {
                sprintln!("PONG {}", line);
                self.client.write(b"JOIN ");
                self.client.write(self.channel.as_bytes());
                self.client.write(b"\nPONG ");
                self.client.write(line.as_bytes());
                self.client.write(b"\n");
            } else if let Some((subcommand, line)) = line.split_once(' ') {
                if subcommand == "001" {
                    self.welcomed = true;
                } else if subcommand == "PRIVMSG" {
                    if let Some((channel, line)) = line.split_once(' ') {
                        if channel == "##flippyflappy" {
                            if let Some(message) = line.strip_prefix(":") {
                                self.next_message = Some(message.to_string());
                            }
                        }
                    }
                }
            }
        }
        self.buffer.clear();
    }
    pub fn take_message(&mut self) -> Option<String> {
        self.next_message.take()
    }
}

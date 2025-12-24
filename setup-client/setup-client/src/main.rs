#![deny(unused_must_use)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![feature(try_blocks)]
mod ble;
mod error;
mod usb;

use crate::ble::{BleAddress, BleConnection};
use crate::error::Error;
use crate::usb::{UsbAddress, UsbConnection};
// use btleplug::api::Peripheral;
use clap::{Parser, ValueEnum};
use crypto_fetch::fetch_certificate_list_sha256;
use futures_util::stream::StreamExt;
use itertools::Itertools;
use jsonformat::Indentation;
use nusb::list_devices;
use nusb::transfer::{Bulk, Direction, Out};
use protocol::setup::{AppSettings, AppStatus, DeviceInfo, MAX_SETUP_MESSAGE_SIZE};
use protocol::setup::{SetupRequest, SetupResponse};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::io::stdin;
use tokio::io::{AsyncReadExt, AsyncWrite};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(long)]
    transport: Transport,
    #[command(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, Clone, ValueEnum)]
enum Transport {
    Usb,
    Ble,
}

#[derive(Parser, Debug)]
enum Subcommand {
    List,
    Read(ReadCommand),
    Write(WriteCommand),
    Info(InfoCommand),
    Monitor(MonitorCommand),
}

#[derive(Parser, Debug)]
struct ReadCommand {
    #[clap(long)]
    address: String,
    #[clap(long)]
    output: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct WriteCommand {
    #[clap(long)]
    address: String,
    #[clap(long)]
    file: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct MonitorCommand {
    #[clap(long)]
    address: String,
}

#[derive(Parser, Debug)]
struct InfoCommand {
    #[clap(long)]
    address: String,
}

enum Connection {
    Usb(UsbConnection),
    Ble(BleConnection),
}

impl Transport {
    async fn connect(&self, address: &str) -> Result<Connection, Error> {
        match self {
            Transport::Usb => Ok(Connection::Usb(UsbConnection::new(address).await?)),
            Transport::Ble => Ok(Connection::Ble(BleConnection::new(address).await?)),
        }
    }
}

impl Connection {
    pub async fn invoke(&mut self, request: &SetupRequest) -> Result<SetupResponse, Error> {
        match self {
            Connection::Usb(conn) => conn.invoke(request).await,
            Connection::Ble(conn) => conn.invoke(request).await,
        }
    }
    pub async fn receive(&mut self) -> Result<AppStatus, Error> {
        match self {
            Connection::Usb(conn) => conn.receive().await,
            Connection::Ble(conn) => conn.receive().await,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();
    match &args.subcommand {
        Subcommand::List => match args.transport {
            Transport::Usb => {
                for display in UsbAddress::list().await? {
                    if let Some(serial) = display.serial_number() {
                        println!("{}", serial);
                    }
                }
            }
            Transport::Ble => {
                let mut list = BleAddress::list().await?;
                while let Some(next) = list.next().await {
                    let next = next?;
                    match next.try_to_string() {
                        Ok(s) => println!("{}", s),
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
            }
        },
        Subcommand::Read(read) => {
            let mut conn = args.transport.connect(&read.address).await?;
            let resp = conn.invoke(&SetupRequest::ReadSettings).await?;
            match resp {
                SetupResponse::ReadSettings(config) => {
                    let content = serde_json_core::to_string::<_, MAX_SETUP_MESSAGE_SIZE>(&config)?;
                    let content = jsonformat::format(&content, Indentation::FourSpace);
                    if let Some(output) = &read.output {
                        fs::write(output, content).await?;
                    } else {
                        println!("{}", content);
                    }
                }
                _ => unreachable!(),
            }
        }
        Subcommand::Write(write) => {
            let mut conn = args.transport.connect(&write.address).await?;
            let settings = if let Some(input) = &write.file {
                fs::read(input).await?
            } else {
                let mut buf = vec![];
                stdin().read_to_end(&mut buf).await?;
                buf
            };
            let mut settings: AppSettings =
                serde_json_core::from_slice_escaped(&settings, &mut [0u8; MAX_SETUP_MESSAGE_SIZE])?
                    .0;
            settings.mqtt.certificate_list_sha256 = Some(
                fetch_certificate_list_sha256(
                    settings.mqtt.hostname.to_string(),
                    settings.mqtt.port,
                )
                .await?,
            );
            let resp = conn.invoke(&SetupRequest::WriteSettings(settings)).await?;
        }
        Subcommand::Monitor(monitor) => {
            let mut conn = args.transport.connect(&monitor.address).await?;
            match conn.invoke(&SetupRequest::TouchAppStatus).await? {
                SetupResponse::TouchAppStatus => {}
                _ => unreachable!(),
            }
            loop {
                println!("status = {:?}", conn.receive().await?);
            }
        }
        Subcommand::Info(info) => {
            let mut conn = args.transport.connect(&info.address).await?;
            match conn.invoke(&SetupRequest::DeviceInfo).await? {
                SetupResponse::DeviceInfo(x) => {
                    let DeviceInfo {
                        serial,
                        git_version,
                        git_dirty,
                        git_head_ref,
                        glyphs,
                        background,
                        foreground,
                    } = x;
                    println!("serial: {:016X}", serial);
                    println!("git_version: {}", git_version);
                    println!("git_dirty: {:?}", git_dirty);
                    println!("git_head_ref: {}", git_head_ref);
                    println!("glyphs: {}", glyphs);
                    println!("background: {}", background);
                    println!("foreground: {}", foreground);
                }
                _ => unreachable!(),
            }
        }
    }
    Ok(())
}

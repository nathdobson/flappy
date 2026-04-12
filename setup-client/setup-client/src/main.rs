#![deny(unused_must_use)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![feature(try_blocks)]
#[cfg(feature = "ble")]
mod ble;
mod error;
#[cfg(feature = "usb")]
mod usb;
#[cfg(feature = "usb")]
mod picoboot;

use crate::error::Error;
// use btleplug::api::Peripheral;
use clap::{Parser, ValueEnum};
use futures_util::stream::StreamExt;
use itertools::Itertools;
use jsonformat::Indentation;
use protocol::setup::{
    AppSettings, AppStatus, DeviceInfo, MAX_SETUP_MESSAGE_SIZE, WriteAppSettings,
};
use protocol::setup::{SetupRequest, SetupResponse};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::io::stdin;
use tokio::io::{AsyncReadExt, AsyncWrite};
use uuid::Uuid;
use crate::picoboot::picoboot;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(long)]
    transport: Transport,
    #[command(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, Clone, ValueEnum, Copy)]
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
    Picoboot(PicobootCommand),
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

#[derive(Parser, Debug)]
struct PicobootCommand {
    #[clap(long)]
    address: String,
    #[clap(long)]
    bin_file: PathBuf,
}

enum Connection {
    #[cfg(feature = "usb")]
    Usb(crate::usb::UsbConnection),
    #[cfg(feature = "ble")]
    Ble(crate::ble::BleConnection),
}

impl Transport {
    async fn connect(&self, address: &str) -> Result<Connection, Error> {
        match self {
            #[cfg(feature = "usb")]
            Transport::Usb => Ok(Connection::Usb(
                crate::usb::UsbConnection::new(address).await?,
            )),
            #[cfg(feature = "ble")]
            Transport::Ble => Ok(Connection::Ble(
                crate::ble::BleConnection::new(address).await?,
            )),
            #[allow(unreachable_patterns)]
            _ => Err(Error::FeatureNotEnabled(*self)),
        }
    }
}

impl Connection {
    pub async fn invoke(&mut self, request: &SetupRequest) -> Result<SetupResponse, Error> {
        match self {
            #[cfg(feature = "usb")]
            Connection::Usb(conn) => conn.invoke(request).await,
            #[cfg(feature = "ble")]
            Connection::Ble(conn) => conn.invoke(request).await,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
    pub async fn receive(&mut self) -> Result<AppStatus, Error> {
        match self {
            #[cfg(feature = "usb")]
            Connection::Usb(conn) => conn.receive().await,
            #[cfg(feature = "ble")]
            Connection::Ble(conn) => conn.receive().await,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();
    match &args.subcommand {
        Subcommand::List => match args.transport {
            #[cfg(feature = "usb")]
            Transport::Usb => {
                for display in crate::usb::UsbAddress::list().await? {
                    if let Some(serial) = display.serial_number() {
                        print!("{}", serial);
                        if display.is_picoboot(){
                            print!(" (awaiting firmware)");
                        }
                        println!();
                    }
                }
            }
            #[cfg(feature = "ble")]
            Transport::Ble => {
                let mut list = crate::ble::BleAddress::list().await?;
                while let Some(next) = list.next().await {
                    let next = next?;
                    match next.try_to_string() {
                        Ok(s) => println!("{}", s),
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
            }
            #[allow(unreachable_patterns)]
            _ => return Err(Error::FeatureNotEnabled(args.transport)),
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
            eprintln!("reading file");
            let mut settings: AppSettings =
                serde_json_core::from_slice_escaped(&settings, &mut [0u8; MAX_SETUP_MESSAGE_SIZE])?
                    .0;
            eprintln!("writing settings");
            let resp = conn
                .invoke(&SetupRequest::WriteSettings(WriteAppSettings {
                    wifi: Some(settings.wifi),
                    mqtt: Some(settings.mqtt),
                    display: Some(settings.display),
                }))
                .await?;
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
        Subcommand::Picoboot(command) => {
            #[cfg(feature = "usb")]
            picoboot(command).await?;
        }
    }
    Ok(())
}

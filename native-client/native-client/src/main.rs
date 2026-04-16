#![deny(unused_must_use)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![feature(try_blocks)]
#![allow(unused_features)]
mod error;
#[cfg(feature = "usb")]
mod picoboot;

// use btleplug::api::Peripheral;
use crate::error::Error;
use clap::{Parser, ValueEnum};
use futures_util::stream::StreamExt;
use itertools::Itertools;
use jsonformat::Indentation;
use protocol::setup::{
    AppSettings, AppStatus, DeviceInfo, MAX_SETUP_MESSAGE_SIZE, WriteAppSettings,
};
use protocol::setup::{SetupRequest, SetupResponse};
use setup_client::client::{Client, ClientTransport};
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
    transport: ClientTransport,
    #[command(subcommand)]
    subcommand: Subcommand,
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

async fn connect(transport: ClientTransport, address: &str) -> Result<Client, Error> {
    match transport {
        #[cfg(feature = "usb")]
        ClientTransport::Usb => Ok(Client::UsbClient(
            setup_client::usb::UsbClientBuilder::list()
                .await?
                .into_iter()
                .find(|x| x.serial_number() == Some(address))
                .ok_or(Error::DeviceNotFound)?
                .connect()
                .await?,
        )),
        #[cfg(feature = "ble")]
        ClientTransport::Ble => {
            let mut scan = setup_client::ble::BleClientBuilder::scan().await?;
            while let Some(next) = scan.next().await {
                let next = next?;
                if next.address() == address {
                    let next = next.connect().await?;
                    eprintln!("Press button on microcontroller.");
                    let next = Client::BleClient(next);
                    next.ping().await?;
                    return Ok(next);
                }
            }
            Err(Error::DeviceNotFound)
        }
        #[allow(unreachable_patterns)]
        _ => Err(Error::FeatureNotEnabled(transport)),
    }
}

#[tokio::main]
async fn main() {
    if let Err(e) = main_impl().await {
        eprintln!("{}", e);
    }
}
async fn main_impl() -> Result<(), Error> {
    let args = Args::parse();
    match &args.subcommand {
        Subcommand::List => match args.transport {
            #[cfg(feature = "usb")]
            ClientTransport::Usb => {
                for x in setup_client::usb::UsbClientBuilder::list().await? {
                    if let Some(sn) = x.serial_number() {
                        println!("{}", sn);
                    } else {
                        println!("<unknown>",);
                    }
                }
            }
            #[cfg(feature = "ble")]
            ClientTransport::Ble => {
                let mut scan = setup_client::ble::BleClientBuilder::scan().await?;
                while let Some(next) = scan.next().await {
                    let next = next?;
                    println!("{}", next.address());
                }
            }
            #[allow(unreachable_patterns)]
            _ => return Err(Error::FeatureNotEnabled(args.transport)),
        },
        Subcommand::Read(read) => {
            let conn = connect(args.transport, &read.address).await?;
            let settings = conn.read_settings().await?;
            let content = serde_json_core::to_string::<_, MAX_SETUP_MESSAGE_SIZE>(&settings)?;
            let content = jsonformat::format(&content, Indentation::FourSpace);
            if let Some(output) = &read.output {
                fs::write(output, content).await?;
            } else {
                println!("{}", content);
            }
        }
        Subcommand::Write(write) => {
            let conn = connect(args.transport, &write.address).await?;
            let settings = if let Some(input) = &write.file {
                fs::read(input).await?
            } else {
                let mut buf = vec![];
                stdin().read_to_end(&mut buf).await?;
                buf
            };
            eprintln!("reading file");
            let settings: AppSettings =
                serde_json_core::from_slice_escaped(&settings, &mut [0u8; MAX_SETUP_MESSAGE_SIZE])?
                    .0;
            eprintln!("writing settings");
            let resp = conn
                .write_settings(WriteAppSettings {
                    wifi: Some(settings.wifi),
                    mqtt: Some(settings.mqtt),
                    display: Some(settings.display),
                })
                .await?;
        }
        Subcommand::Monitor(monitor) => {
            let conn = connect(args.transport, &monitor.address).await?;
            conn.touch_app_status().await?;
            loop {
                println!("status = {:?}", conn.receive_status().await?);
            }
        }
        Subcommand::Info(info) => {
            let conn = connect(args.transport, &info.address).await?;
            let DeviceInfo {
                serial,
                git_version,
                git_dirty,
                git_head_ref,
                glyphs,
                background,
                foreground,
            } = conn.device_info().await?;
            println!("serial: {:016X}", serial);
            println!("git_version: {}", git_version);
            println!("git_dirty: {:?}", git_dirty);
            println!("git_head_ref: {}", git_head_ref);
            println!("glyphs: {}", glyphs);
            println!("background: {}", background);
            println!("foreground: {}", foreground);
        }
        Subcommand::Picoboot(command) => {
            #[cfg(feature = "usb")]
            crate::picoboot::picoboot(command).await?;
        }
    }
    Ok(())
}

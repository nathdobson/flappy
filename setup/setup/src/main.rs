#![deny(unused_must_use)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
mod display;
mod error;

use crate::display::{DisplayInfo, DisplaySetup, DisplayStatus};
use crate::error::Error;
use clap::Parser;
use itertools::Itertools;
use jsonformat::Indentation;
use nusb::list_devices;
use nusb::transfer::{Bulk, Direction, Out};
use proto::setup::MAX_SETUP_MESSAGE_SIZE;
use proto::setup::{CUSTOM_CLASS_ID, SetupRequest, SetupResponse};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::io::stdin;
use tokio::io::{AsyncReadExt, AsyncWrite};
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    subcommand: Subcommand,
}

#[derive(Parser, Debug)]
enum Subcommand {
    List,
    Read(ReadCommand),
    Write(WriteCommand),
    Monitor(MonitorCommand),
}

#[derive(Parser, Debug)]
struct ReadCommand {
    serial: String,
    output: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct WriteCommand {
    serial: String,
    input: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct MonitorCommand {
    serial: String,
}

async fn connect(serial: &str) -> Result<(DisplaySetup, DisplayStatus), Error> {
    let list = DisplayInfo::list().await?;
    match list
        .iter()
        .filter(|x| x.serial_number() == Some(&serial))
        .exactly_one()
    {
        Ok(found) => Ok(found.connect().await?),
        Err(mut e) => {
            if e.next().is_some() {
                Err(Error::DuplicateSerialNumber)
            } else {
                Err(Error::DisplayNotFound)
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();
    match &args.subcommand {
        Subcommand::List => {
            for display in DisplayInfo::list().await? {
                if let Some(serial) = display.serial_number() {
                    println!("{}", serial);
                }
            }
        }
        Subcommand::Read(read) => {
            let (mut setup, _) = connect(&read.serial).await?;
            let resp = setup.invoke(&SetupRequest::ReadSettings).await?;
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
            let (mut setup, _) = connect(&write.serial).await?;
            let settings = if let Some(input) = &write.input {
                fs::read(input).await?
            } else {
                let mut buf = vec![];
                stdin().read_to_end(&mut buf).await?;
                buf
            };
            let settings =
                serde_json_core::from_slice_escaped(&settings, &mut [0u8; MAX_SETUP_MESSAGE_SIZE])?
                    .0;
            let resp = setup.invoke(&SetupRequest::WriteSettings(settings)).await?;
        }
        Subcommand::Monitor(monitor) => {
            let (mut setup, mut status) = connect(&monitor.serial).await?;
            setup.invoke(&SetupRequest::TouchAppStatus).await?;
            loop {
                println!("status = {:?}", status.receive().await?);
            }
        }
    }
    Ok(())
}

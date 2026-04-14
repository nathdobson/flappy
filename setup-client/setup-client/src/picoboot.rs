use crate::PicobootCommand;
use crate::error::Error;
use itertools::Itertools;
use nusb::list_devices;
use nusb::transfer::TransferError;
use picoboot::{Access, Picoboot};
use setup_client_lib::usb::{BootSelect, UsbClientBuilder};
use std::time::Duration;
use tokio::fs;

pub async fn picoboot(command: &PicobootCommand) -> Result<(), Error> {
    eprintln!("Searching for devices...");
    let mut builder = UsbClientBuilder::list()
        .await?
        .into_iter()
        .find(|x| x.serial_number() == Some(&command.address))
        .ok_or(Error::DeviceNotFound)?;

    if builder.boot_select() == BootSelect::Application {
        eprintln!("Found device in Application mode. Rebooting in Picoboot mode.");
        if let Err(e) = builder.connect().await?.reset_picoboot().await {
            match e {
                setup_client_lib::error::Error::TransferError(TransferError::Unknown(
                    0xe00002ed,
                )) => {}
                _ => eprintln!(
                    "Notice: error when rebooting device. This may be nothing. {}",
                    e
                ),
            }
        }
        tokio::time::sleep(Duration::from_millis(1000)).await;
        eprintln!("Reconnecting...");
        builder = UsbClientBuilder::list()
            .await?
            .into_iter()
            .find(|x| x.serial_number() == Some(&command.address))
            .ok_or(Error::DeviceNotFound)?;
        eprintln!("Reconnected.");
    }
    if builder.boot_select() != BootSelect::Picoboot {
        return Err(Error::RebootFailed);
    }
    eprintln!("Connecting to picoboot interface...");
    let mut picoboot = builder.connect_picoboot().await?;
    eprintln!("Establishing picoboot connection...");
    let conn = picoboot.connect().await?;
    eprintln!("obtaining exclusive access");
    conn.set_exclusive_access(Access::ExclusiveAndEject).await?;
    eprintln!("Exiting xip");
    conn.exit_xip().await?;
    let binary = fs::read(&command.bin_file).await?;
    eprintln!("Erasing...");
    conn.flash_erase_start(binary.len()).await?;
    eprintln!("Writing...");
    conn.flash_write_start(&binary).await?;
    eprintln!("Verifying...");
    let verified = conn.flash_read_start(binary.len() as u32).await?;
    if binary != verified {
        println!(
            "Verification failure ({} bytes do not match {} bytes)",
            binary.len(),
            verified.len()
        );
    }
    conn.reboot(Duration::from_millis(500)).await?;
    Ok(())
}

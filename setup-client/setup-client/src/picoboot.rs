use std::time::Duration;
use crate::PicobootCommand;
use crate::error::Error;
use crate::usb::UsbAddress;
use itertools::Itertools;
use picoboot::{Access, Picoboot};
use tokio::fs;

pub async fn picoboot(command: &PicobootCommand) -> Result<(), Error> {
    let list = UsbAddress::list().await?;
    let found = match list
        .iter()
        .filter(|x| x.serial_number() == Some(&command.address))
        .exactly_one()
    {
        Ok(found) => found,
        Err(mut e) => {
            if e.next().is_some() {
                return Err(Error::DuplicateSerialNumber);
            } else {
                return Err(Error::DisplayNotFound);
            }
        }
    };
    let mut picoboot = Picoboot::from_first(None).await?;
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

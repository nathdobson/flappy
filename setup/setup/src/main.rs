#![deny(unused_must_use)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
mod error;

use crate::error::Error;
use nusb::list_devices;
use nusb::transfer::{Bulk, Direction, Out};
use proto::CUSTOM_CLASS_ID;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> Result<(), Error> {
    for dev in list_devices().await? {
        if dev.vendor_id() == proto::VENDOR_ID && dev.product_id() == proto::PRODUCT_ID {
            println!("Connecting to {:?}", dev.serial_number());
            let dev = dev.open().await?;
            let config = dev.active_configuration()?;
            for int in config.interfaces() {
                if int.first_alt_setting().class() == CUSTOM_CLASS_ID {
                    let int = dev.claim_interface(int.interface_number()).await?;
                    if let Some(desc) = int.descriptor() {
                        for ep in desc.endpoints() {
                            match ep.direction() {
                                Direction::Out => {
                                    let ep = int.endpoint::<Bulk, Out>(ep.address())?;
                                    let mut ep = ep.writer(64);
                                    ep.write(b"Hello Flappy!").await?;
                                    ep.flush().await?;
                                }
                                Direction::In => {}
                            }
                        }
                    }
                    println!("{:?}", int);
                }
            }
            println!("{:?}", dev);
        }
    }
    Ok(())
}

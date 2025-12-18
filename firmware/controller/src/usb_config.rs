use core::mem;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, Endpoint, In, Out};
use embassy_usb::Builder;
use embassy_usb::driver::{EndpointIn, EndpointOut};
use log::{error, info};
use proto::CUSTOM_CLASS_ID;
use crate::error::Error;

pub struct UsbConfig {}

impl UsbConfig {
    pub fn new(
        builder: &mut Builder<'static, Driver<'static, USB>>,
        spawner: Spawner,
    ) -> Result<(), Error> {
        let mut custom = builder.function(CUSTOM_CLASS_ID, 0x00, 0x00);
        let mut custom_if = custom.interface();
        let mut alt = custom_if.alt_setting(CUSTOM_CLASS_ID, 0x00, 0x00, None);
        let in_ep = alt.endpoint_bulk_in(None, crate::usb::MAX_PACKET_SIZE as u16);
        let out_ep = alt.endpoint_bulk_out(None, crate::usb::MAX_PACKET_SIZE as u16);
        spawner.spawn({
            #[embassy_executor::task]
            async fn send_task(mut in_ep: Endpoint<'static, USB, In>) {
                if let Err(e) = in_ep.write(b"Hello").await {
                    error!("{:?}", e);
                }
            }
            send_task(in_ep)?
        });
        spawner.spawn({
            #[embassy_executor::task]
            async fn receive_task(mut out_ep: Endpoint<'static, USB, Out>) {
                let mut buf = [0u8; 128];
                loop {
                    match out_ep.read(&mut buf).await {
                        Ok(x) => info!("Received {:?}", &buf[..x]),
                        Err(e) => error!("{:?}", e),
                    }
                }
            }
            receive_task(out_ep)?
        });
        mem::drop(custom);
        Ok(())
    }
}

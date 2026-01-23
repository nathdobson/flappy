use crate::error::Error;
use crate::make_static;
use crate::product::serial_number;
use crate::runtime::RuntimePeripherals;
use crate::usb_reset::UsbResetModule;
use crate::usb_serial::UsbSerialModule;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_rp::{Peri, bind_interrupts};
use embassy_usb::{Builder, Config, UsbDevice};
use log::error;
use protocol::usb::{PRODUCT_ID, VENDOR_ID};
use static_cell::StaticCell;

pub const MAX_PACKET_SIZE: u8 = 64;

pub struct UsbModule {
    pub usb_serial: &'static UsbSerialModule,
    pub usb_reset: &'static UsbResetModule,
    #[cfg(feature = "setup")]
    pub usb_setup: &'static crate::usb_setup::UsbSetupModule,
}

struct RuntimeState {
    config_descriptor: [u8; 128],
    bos_descriptor: [u8; 16],
    msos_descriptor: [u8; 256],
    control_buf: [u8; 64],
}

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
});

impl UsbModule {
    pub fn new() -> &'static Self {
        let module = make_static!(
            UsbModule,
            UsbModule {
                usb_serial: UsbSerialModule::new(),
                usb_reset: UsbResetModule::new(),
                #[cfg(feature = "setup")]
                usb_setup: crate::usb_setup::UsbSetupModule::new(),
            }
        );
        module
    }
    pub async fn start(
        &'static self,
        spawner: Spawner,
        peri: Peri<'static, USB>,
    ) -> Result<(), Error> {
        let driver = Driver::new(peri, UsbIrqs);

        let mut config = Config::new(VENDOR_ID, PRODUCT_ID);
        config.manufacturer = Some(protocol::PRODUCT_MANUFACTURER);
        config.product = Some(protocol::PRODUCT_NAME);

        config.serial_number = serial_number();
        config.max_power = 100;
        config.max_packet_size_0 = MAX_PACKET_SIZE;

        config.device_class = 0xef;
        config.device_sub_class = 0x02;
        config.device_protocol = 0x01;
        config.composite_with_iads = true;

        let state: &'static mut RuntimeState = make_static!(
            RuntimeState,
            RuntimeState {
                config_descriptor: [0; 128],
                bos_descriptor: [0; 16],
                msos_descriptor: [0; 256],
                control_buf: [0; 64],
            }
        );
        let mut builder = Builder::new(
            driver,
            config,
            &mut state.config_descriptor,
            &mut state.bos_descriptor,
            &mut state.msos_descriptor,
            &mut state.control_buf,
        );
        if let Err(e) = self.usb_serial.start(spawner, &mut builder).await {
            error!("Failed to start USB serial: {:?}", e);
        }
        if let Err(e) = self.usb_reset.start(spawner, &mut builder).await {
            error!("Failed to start USB reset: {:?}", e);
        }
        #[cfg(feature = "setup")]
        if let Err(e) = self.usb_setup.start(spawner, &mut builder).await {
            error!("Failed to start USB config: {:?}", e);
        }
        let mut device = builder.build();
        spawner.spawn({
            #[embassy_executor::task]
            async fn device_task(mut device: UsbDevice<'static, Driver<'static, USB>>) -> ! {
                device.run().await
            }
            device_task(device)?
        });
        Ok(())
    }
}

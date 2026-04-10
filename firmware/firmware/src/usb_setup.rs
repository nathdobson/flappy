use crate::error::Error;
use crate::make_static;
use crate::usb::MAX_PACKET_SIZE;
use core::mem;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, Endpoint, In, Out};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, DynamicReceiver, DynamicSender};
use embassy_sync::signal::Signal;
use embassy_sync::watch::{DynReceiver, DynSender, Watch};
use embassy_usb::Builder;
use embassy_usb::descriptor::{SynchronizationType, UsageType};
use embassy_usb::driver::Endpoint as _;
use embassy_usb::driver::{EndpointError, EndpointIn, EndpointOut, EndpointType};
use log::{error, info};
use protocol::setup::{
    AppStatus, MAX_SETUP_MESSAGE_SIZE, MqttServiceStatus, SetupRequest, SetupResponse,
};
use protocol::usb::{CUSTOM_CLASS_ID, CUSTOM_SUBCLASS_ID};

pub struct UsbSetupModule {
    setup_request: Channel<CriticalSectionRawMutex, SetupRequest, 1>,
    setup_response: Channel<CriticalSectionRawMutex, SetupResponse, 1>,
    setup_status: Watch<CriticalSectionRawMutex, AppStatus, 1>,
}

impl UsbSetupModule {
    pub fn new() -> &'static Self {
        make_static!(
            UsbSetupModule,
            UsbSetupModule {
                setup_request: Channel::new(),
                setup_response: Channel::new(),
                setup_status: Watch::new(),
            }
        )
    }
    pub async fn start(
        &'static self,
        spawner: Spawner,
        builder: &mut Builder<'static, Driver<'static, USB>>,
    ) -> Result<(), Error> {
        let mut custom = builder.function(CUSTOM_CLASS_ID, CUSTOM_SUBCLASS_ID, 0x00);
        let mut custom_if = custom.interface();
        let mut alt = custom_if.alt_setting(CUSTOM_CLASS_ID, CUSTOM_SUBCLASS_ID, 0x00, None);
        let in_ep = alt.endpoint_bulk_in(None, MAX_PACKET_SIZE as u16);
        let out_ep = alt.endpoint_bulk_out(None, MAX_PACKET_SIZE as u16);
        let status_ep = alt.endpoint_bulk_in(None, MAX_PACKET_SIZE as u16);
        spawner.spawn({
            #[embassy_executor::task]
            async fn send_responses_task(
                module: &'static UsbSetupModule,
                mut in_ep: Endpoint<'static, USB, In>,
            ) {
                if let Err(e) = module.send_responses(in_ep).await {
                    error!("{:?}", e);
                }
            }
            send_responses_task(self, in_ep)?
        });
        spawner.spawn({
            #[embassy_executor::task]
            async fn send_status_task(
                module: &'static UsbSetupModule,
                mut in_ep: Endpoint<'static, USB, In>,
            ) {
                if let Err(e) = module.send_status(in_ep).await {
                    error!("{:?}", e);
                }
            }
            send_status_task(self, status_ep)?
        });
        spawner.spawn({
            #[embassy_executor::task]
            async fn receive_requests_task(
                module: &'static UsbSetupModule,
                mut out_ep: Endpoint<'static, USB, Out>,
            ) {
                if let Err(e) = module.receive_requests(out_ep).await {
                    error!("{:?}", e);
                }
            }
            receive_requests_task(self, out_ep)?
        });
        mem::drop(custom);
        Ok(())
    }
    async fn receive_requests(&self, mut out_ep: Endpoint<'static, USB, Out>) -> Result<(), Error> {
        let mut buf = [0u8; MAX_SETUP_MESSAGE_SIZE];
        let mut tmp = [0; MAX_SETUP_MESSAGE_SIZE];
        loop {
            out_ep.wait_enabled().await;
            self.setup_status.sender().send_modify(|x| {});
            loop {
                match out_ep.read_transfer(&mut buf).await {
                    Ok(len) => {
                        self.setup_request
                            .send(
                                serde_json_core::from_slice_escaped::<SetupRequest>(
                                    &buf[..len],
                                    &mut tmp,
                                )?
                                .0,
                            )
                            .await;
                    }
                    Err(e) => match e {
                        EndpointError::BufferOverflow => {
                            error!("Buffer overflow when receiving setup request");
                            continue;
                        }
                        EndpointError::Disabled => {
                            break;
                        }
                    },
                }
            }
        }
        Ok(())
    }
    async fn send_responses(&self, mut in_ep: Endpoint<'static, USB, In>) -> Result<(), Error> {
        let mut buf;
        loop {
            let response = self.setup_response.receive().await;
            buf = serde_json_core::to_vec::<_, MAX_SETUP_MESSAGE_SIZE>(&response)?;
            in_ep.write_transfer(&buf, true).await?;
        }
        Ok(())
    }
    async fn send_status(&self, mut in_ep: Endpoint<'static, USB, In>) -> Result<(), Error> {
        let mut buf;
        let mut receiver = self
            .setup_status
            .receiver()
            .ok_or(Error::NotEnoughReceivers)?;
        loop {
            let status = receiver.changed().await;
            info!("Sending status {:?}", status);
            in_ep.wait_enabled().await;
            buf = serde_json_core::to_vec::<_, MAX_SETUP_MESSAGE_SIZE>(&status)?;
            in_ep.write_transfer(&buf, true).await?;
            info!("Finished sending status");
        }
        Ok(())
    }
    pub fn update_status(&self, f: impl Fn(&mut AppStatus)) {
        self.setup_status
            .sender()
            .send_modify(move |x| f(x.get_or_insert_default()))
    }
    pub fn requests(&'static self) -> DynamicReceiver<'static, SetupRequest> {
        self.setup_request.dyn_receiver()
    }
    pub fn responses(&'static self) -> DynamicSender<'static, SetupResponse> {
        self.setup_response.dyn_sender()
    }
}

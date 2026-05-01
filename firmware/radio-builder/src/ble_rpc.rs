use crate::Error;
use crate::ble::{AdvertiseBuilder, MyGattConnection, MyPeripheral};
use alloc::boxed::Box;
use core::cell::RefCell;
use core::marker::PhantomData;
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel;
use embassy_sync::watch::Watch;
use fixed_freelist::{Freelist, FreelistStorage};
use heapless::{String, Vec};
use log::warn;
use protocol_ble::SERIAL_MTU;
use protocol_ble::trouble_host::{
    APP_STATUS_UUID, RPC_SERVICE_UUID, SERIAL_IN_UUID, SERIAL_OUT_UUID,
};
use serde::{Deserialize, Serialize};
use trouble_host::attribute::Characteristic;
use trouble_host::gap::{GapConfig, PeripheralConfig};
use trouble_host::gatt::{GattConnectionEvent, GattEvent};
use trouble_host::prelude::FromGatt;
use trouble_host::prelude::{appearance, descriptors, gatt_server, gatt_service};

#[gatt_server]
struct RpcServer {
    rpc_service: RpcService,
}

const STATUS_SIZE: usize = 256;

#[gatt_service(uuid = RPC_SERVICE_UUID)]
struct RpcService {
    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "Serial in")]
    #[characteristic(
        uuid = SERIAL_IN_UUID,
        read,
        notify,
        permissions(encrypted)
    )]
    serial_in: Vec<u8, SERIAL_MTU>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "Serial out")]
    #[characteristic(
        uuid = SERIAL_OUT_UUID,
        write,
        permissions(encrypted)
    )]
    serial_out: Vec<u8, SERIAL_MTU>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "App Status")]
    #[characteristic(uuid = APP_STATUS_UUID, read, notify)]
    app_status: String<STATUS_SIZE>,
}

pub const RPC_SLOTS: usize = 10;

pub struct RpcAdvertiser<Req: 'static, Resp: 'static, Stat: 'static + Clone> {
    short_name: &'static str,
    peri: RefCell<MyPeripheral<RPC_SLOTS>>,
    server: RpcServer<'static>,
    out_allocator: FreelistStorage<NoopRawMutex, Req, 1>,
    status: Watch<NoopRawMutex, Stat, 1>,
    phantom: PhantomData<Resp>,
}

pub struct RpcConnection<Req: 'static, Resp: 'static, Stat: 'static + Clone, const BUFFER: usize> {
    advertiser: &'static RpcAdvertiser<Req, Resp, Stat>,
    gatt: MyGattConnection,
    out_channel: channel::Channel<NoopRawMutex, Box<Req, Freelist<'static, NoopRawMutex>>, 1>,
}

impl<Req, Resp, Stat: 'static + Clone + Default> RpcAdvertiser<Req, Resp, Stat> {
    pub fn new(
        peri: MyPeripheral<RPC_SLOTS>,
        short_name: &'static str,
        long_name: &'static str,
    ) -> Result<Self, Error> {
        Ok(RpcAdvertiser {
            short_name,
            peri: RefCell::new(peri),
            server: RpcServer::new_with_config(GapConfig::Peripheral(PeripheralConfig {
                name: long_name,
                appearance: &appearance::domestic_appliance::COFFEE_MAKER,
            }))
            .map_err(Error::GattConfigError)?,
            out_allocator: FreelistStorage::new(),
            status: Watch::new(),
            phantom: Default::default(),
        })
    }
    pub async fn advertise<const BUFFER: usize>(
        &'static self,
    ) -> Result<RpcConnection<Req, Resp, Stat, BUFFER>, Error> {
        let mut peri = self.peri.borrow_mut();
        let conn = AdvertiseBuilder {
            peri: &mut *peri,
            service: RPC_SERVICE_UUID,
            short_name: self.short_name,
        }
        .build()
        .await?;
        let gatt = conn.with_attribute_server(&self.server)?;
        Ok(RpcConnection {
            advertiser: self,
            gatt,
            out_channel: channel::Channel::new(),
        })
    }

    pub fn update_status<F: Fn(&mut Stat)>(&self, f: F) {
        self.status
            .sender()
            .send_modify(|x| f(x.get_or_insert_default()));
    }
}

impl<
    Req: for<'de> Deserialize<'de>,
    Resp: Serialize,
    Stat: 'static + Clone + Default + Serialize,
    const BUFFER: usize,
> RpcConnection<Req, Resp, Stat, BUFFER>
{
    pub async fn run(&self) -> Result<(), Error> {
        match select(self.receive_events(), self.send_statuses()).await {
            Either::First(x) => x?,
            Either::Second(x) => x?,
        }
        Ok(())
    }
    async fn receive_events(&self) -> Result<(), Error> {
        let mut tmp = [0u8; BUFFER];
        let mut buffer = Vec::<u8, BUFFER>::new();
        loop {
            match self.gatt.next().await {
                GattConnectionEvent::Gatt { event } => {
                    match &event {
                        GattEvent::Write(event) => {
                            if event.handle()
                                == self.advertiser.server.rpc_service.serial_out.handle
                            {
                                let data =
                                    event.value(&self.advertiser.server.rpc_service.serial_out)?;
                                buffer
                                    .extend_from_slice(&data)
                                    .map_err(|_| Error::RequestTooLarge)?;
                                if data.len() < SERIAL_MTU {
                                    let req = serde_json_core::from_slice_escaped::<Req>(
                                        &buffer, &mut tmp,
                                    )?
                                    .0;
                                    buffer.clear();
                                    let req = self
                                        .advertiser
                                        .out_allocator
                                        .alloc_box(req)
                                        .map_err(|_| Error::ConcurrentRequests)?;
                                    self.out_channel
                                        .try_send(req)
                                        .map_err(|_| Error::ConcurrentRequests)?;
                                }
                            }
                        }
                        _ => {}
                    }
                    match event.accept() {
                        Ok(reply) => reply.send().await,
                        Err(e) => warn!("error sending response: {:?}", e),
                    }
                }
                GattConnectionEvent::Disconnected { reason } => {
                    reason.to_result()?;
                }
                _ => {}
            }
        }
    }
    async fn send_statuses(&self) -> Result<!, Error> {
        let status = self.advertiser.status.try_get().unwrap_or_default();
        self.send_status(&status).await?;
        let mut receiver = self.advertiser.status.receiver().unwrap();
        loop {
            let status = receiver.changed().await;
            self.send_status(&status).await?;
        }
    }
    pub async fn receive(&self) -> Box<Req, Freelist<'static, NoopRawMutex>> {
        self.out_channel.receive().await
    }
    pub async fn send(&self, resp: &Resp) -> Result<(), Error> {
        let data = serde_json_core::to_vec::<_, BUFFER>(&resp)?;
        self.send_bulk(&self.advertiser.server.rpc_service.serial_in, &data)
            .await?;
        Ok(())
    }
    async fn send_status(&self, status: &Stat) -> Result<(), Error> {
        let status = serde_json_core::to_string::<_, STATUS_SIZE>(status)?;
        self.advertiser
            .server
            .rpc_service
            .app_status
            .notify(&self.gatt, &status)
            .await?;
        Ok(())
    }
    async fn send_bulk(
        &self,
        attr: &Characteristic<Vec<u8, SERIAL_MTU>>,
        data: &[u8],
    ) -> Result<(), Error> {
        for chunk in data.chunks(SERIAL_MTU) {
            let chunk = Vec::<u8, SERIAL_MTU>::from_slice(chunk).unwrap();
            attr.notify(&self.gatt, &chunk).await?;
        }
        if data.len() % SERIAL_MTU == 0 {
            attr.notify(&self.gatt, &Vec::new()).await?;
        }
        Ok(())
    }
}

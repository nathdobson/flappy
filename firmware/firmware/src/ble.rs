use crate::bootsel::BootselModule;
use crate::error::Error;
use board_info::serial_number;
use core::cell::{Cell, RefCell};
use core::future::{join, pending};
use core::mem;
use core::str::Utf8Error;
use cyw43::bluetooth::BtDriver;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, Either3, select, select3};
use embassy_futures::yield_now;
use embassy_rp::clocks::RoscRng;
use embassy_rp::otp::get_chipid;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embassy_sync::channel::{Channel, DynamicReceiver, DynamicSender};
use embassy_sync::signal::Signal;
use embassy_sync::watch::Watch;
use embassy_time::{Delay, Duration, Timer, WithTimeout};
use embedded_hal_async::delay::DelayNs;
use heapless::{String, Vec, format};
use log::{error, info, warn};
use make_static::make_static;
use protocol::setup::{AppStatus, MAX_SETUP_MESSAGE_SIZE, SetupRequest, SetupResponse};
use protocol::{PRODUCT_NAME, PRODUCT_SHORT_NAME};
use protocol_ble::SERIAL_MTU;
use radio_builder::ble::{BleBuilder, BlePeripherals, BleStack};
use radio_builder::ble_rpc::{RpcAdvertiser, RpcConnection};
use runtime::LocalSpawn;
use trouble_host::advertise::{
    AdStructure, Advertisement, AdvertisementParameters, BR_EDR_NOT_SUPPORTED,
    LE_GENERAL_DISCOVERABLE, PhyKind, TxPower,
};
use trouble_host::attribute::{AttributeTable, Characteristic, Uuid};
use trouble_host::connection::{Connection, SecurityLevel};
use trouble_host::gap::{GapConfig, PeripheralConfig};
use trouble_host::gatt::{GattConnection, GattConnectionEvent, GattEvent};
use trouble_host::l2cap::{CreditFlowPolicy, L2capChannel, L2capChannelConfig};
use trouble_host::prelude::{
    AddrKind, AsGatt, AttributeServer, BdAddr, CccdTable, DefaultPacketPool, ExternalController,
    FromGatt, HeaplessString, Peripheral, Runner, appearance, descriptors, gatt_server,
    gatt_service, uuid,
};
use trouble_host::{Address, HostResources, IoCapabilities, PacketPool, Stack, advertise};

const MODULE: &'static str = "[BLE  ]";

const SLOTS: usize = 10;
pub struct BleModule {
    advertiser: &'static RpcAdvertiser<SetupRequest, SetupResponse, AppStatus>,
    bootsel: &'static BootselModule,
}

impl BleModule {
    pub fn new(
        spawner: Spawner,
        ble: BlePeripherals,
        bootsel: &'static BootselModule,
    ) -> Result<&'static BleModule, Error> {
        let ble = BleBuilder {
            peripherals: ble,
            spawner,
            stack: make_static!(
                BleStack::<SLOTS, /*CONNS*/ 1, /*CHANNELS*/ 2, 1, 10>,
                BleStack::new()
            ),
        }
        .build()?;
        let name = make_static!(
            String<28>,
            format!("FLAP {}", serial_number().unwrap_or("<noid>"))?
        );

        let advertiser = make_static!(
            _,
            RpcAdvertiser::<SetupRequest, SetupResponse, AppStatus>::new(
                ble,
                PRODUCT_SHORT_NAME,
                name
            )?
        );
        let module = make_static!(
            BleModule,
            BleModule {
                advertiser,
                bootsel
            }
        );

        Ok(module)
    }
    pub async fn advertise<H: AsyncFn(&SetupRequest) -> SetupResponse>(
        &self,
        handler: H,
    ) -> Result<(), Error> {
        loop {
            let connection = self.advertiser.advertise().await?;
            let result = select(
                self.run_connection(&connection),
                self.run_requests(&connection, &handler),
            )
            .await;
            let result = match result {
                Either::First(x) => x,
                Either::Second(x) => x.map(|x| ()),
            };
            if let Err(e) = result {
                error!("Error during BLE connection: {}", e);
            }
        }
    }
    async fn run_connection(
        &self,
        connection: &RpcConnection<SetupRequest, SetupResponse, AppStatus, MAX_SETUP_MESSAGE_SIZE>,
    ) -> Result<(), Error> {
        connection.run().await?;
        Ok(())
    }
    async fn run_requests<H: AsyncFn(&SetupRequest) -> SetupResponse>(
        &self,
        connection: &RpcConnection<SetupRequest, SetupResponse, AppStatus, MAX_SETUP_MESSAGE_SIZE>,
        handler: &H,
    ) -> Result<!, Error> {
        let mut bootsel_pressed = false;
        loop {
            let request = connection.receive().await;
            if !bootsel_pressed {
                loop {
                    if self.bootsel.is_pressed() {
                        bootsel_pressed = true;
                        break;
                    } else {
                        Delay.delay_ms(100).await;
                    }
                }
            }
            let response = handler(&request).await;
            connection.send(&response).await?;
        }
    }

    pub fn update_status(&self, f: impl Fn(&mut AppStatus)) {
        self.advertiser.update_status(f);
    }
}

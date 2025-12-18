use crate::ble_gatt::{FLAPPY_SERVICE_UUID, Server};
use crate::error::Error;
use crate::product;
use core::cell::{Cell, RefCell};
use core::future::{join, pending};
use core::mem;
use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_futures::yield_now;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use log::{error, info, warn};
use static_cell::make_static;
use trouble_host::advertise::{
    AdStructure, Advertisement, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
};
use trouble_host::attribute::{AttributeTable, Characteristic, Uuid};
use trouble_host::gap::{GapConfig, PeripheralConfig};
use trouble_host::gatt::{GattConnection, GattConnectionEvent, GattEvent};
use trouble_host::prelude::{
    AsGatt, CccdTable, DefaultPacketPool, ExternalController, FromGatt, Peripheral, Runner,
    appearance,
};
use trouble_host::{HostResources, PacketPool, Stack};
use cyw43::bluetooth::BtDriver;
const MODULE: &'static str = "[BLE  ]";
/// Max number of connections
const CONNECTIONS_MAX: usize = 1;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 2; // Signal + att

const SLOTS: usize = 10;

pub struct BleModule {
    spawner: Spawner,
    server: MyServer,
    stack: &'static MyStack,
    conn: RefCell<Option<MyConnection>>,
    peri: Cell<Option<MyPeripheral>>,
}

type MyPacketPool = DefaultPacketPool;
type MyDriver = BtDriver<'static>;
pub type MyController = ExternalController<MyDriver, SLOTS>;
type MyResources = HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>;
type MyPeripheral = Peripheral<'static, MyController, MyPacketPool>;
type MyStack = Stack<'static, MyController, MyPacketPool>;
type MyRunner = Runner<'static, MyController, MyPacketPool>;

type MyServer = Server<'static>;

type MyConnection = GattConnection<'static, 'static, MyPacketPool>;

pub trait BleHandler {
    fn ble_handle_gatt_write(&self, id: u16);
}

impl BleModule {
    pub async fn new(spawner: Spawner, driver: MyDriver) -> Result<&'static BleModule, Error> {
        info!("{MODULE} Starting Bluetooth Low Energy");
        let controller = MyController::new(driver);
        let resources: &mut MyResources = make_static!(MyResources::new());
        let stack: &MyStack = make_static!(trouble_host::new(controller, resources));
        let mut host = stack.build();
        let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
            name: proto::PRODUCT_NAME,
            appearance: &appearance::domestic_appliance::COFFEE_MAKER,
        }))?;
        let module: &BleModule = make_static!(BleModule {
            spawner,
            server,
            stack,
            conn: RefCell::new(None),
            peri: Cell::new(Some(host.peripheral)),
        });
        spawner.clone().spawn({
            #[embassy_executor::task]
            async fn ble_task(mut runner: MyRunner) {
                if let Err(e) = runner.run().await {
                    error!("{MODULE} system error: {:?}", e);
                }
            }
            ble_task(host.runner)?
        });

        info!("{MODULE} Started");

        Ok(module)
    }
    pub fn start(&'static self, on_write: &'static dyn BleHandler) -> Result<(), Error> {
        self.spawner.clone().spawn({
            #[embassy_executor::task]
            async fn advertise_task(module: &'static BleModule, on_write: &'static dyn BleHandler) {
                module.advertise_loop(on_write).await;
            }
            advertise_task(self, on_write)?
        });
        Ok(())
    }
    async fn advertise_loop(&'static self, on_write: &'static dyn BleHandler) {
        let Some(mut peripheral) = self.peri.take() else {
            return;
        };
        loop {
            match self.advertise(&mut peripheral).await {
                Ok(conn) => {
                    self.conn.borrow_mut().replace(conn);
                    if let Err(e) = self
                        .handle_connection(self.conn.borrow().as_ref().unwrap(), on_write)
                        .await
                    {
                        error!("{MODULE} error while processing connection: {}", e)
                    } //
                    self.conn.borrow_mut().take();
                }
                Err(e) => {
                    error!("{MODULE} error while advertising: {}", e);
                }
            }
        }
    }
    pub async fn advertise(
        &'static self,
        peripheral: &mut MyPeripheral,
    ) -> Result<GattConnection<'static, 'static, DefaultPacketPool>, Error> {
        let mut advertiser_data = [0; 128];
        const FLAPPY_SERVICE_UUID_BYTES: [u8; 16] = {
            match FLAPPY_SERVICE_UUID {
                Uuid::Uuid16(_) => unreachable!(),
                Uuid::Uuid128(x) => x,
            }
        };
        let mut service_uuid = FLAPPY_SERVICE_UUID_BYTES;
        // service_uuid.reverse();
        let len = AdStructure::encode_slice(
            &[
                AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                AdStructure::ServiceUuids16(&[[0x0f, 0x18]]),
                AdStructure::ServiceUuids128(&[service_uuid]),
                AdStructure::CompleteLocalName(proto::PRODUCT_NAME.as_bytes()),
            ],
            &mut advertiser_data[..],
        )?;
        let advertiser = peripheral
            .advertise(
                &Default::default(),
                Advertisement::ConnectableScannableUndirected {
                    adv_data: &advertiser_data[..len],
                    scan_data: &[],
                },
            )
            .await?;
        let conn = advertiser
            .accept()
            .await?
            .with_attribute_server(&self.server)?;
        info!("{MODULE} connection established");
        Ok(conn)
    }
    async fn handle_connection(
        &'static self,
        conn: &MyConnection,
        on_write: &dyn BleHandler,
    ) -> Result<(), Error> {
        loop {
            match conn.next().await {
                GattConnectionEvent::Disconnected { reason } => {
                    info!("{MODULE} disconnected from peer: {:?}", reason);
                    break;
                }
                GattConnectionEvent::Gatt { event } => {
                    let mut written_handle = None;
                    match &event {
                        GattEvent::Read(event) => {}
                        GattEvent::Write(event) => {
                            written_handle = Some(event.handle());
                        }
                        _ => {}
                    };
                    match event.accept() {
                        Ok(reply) => reply.send().await,
                        Err(e) => warn!("{MODULE} error sending response: {:?}", e),
                    };
                    if let Some(do_handle) = written_handle {
                        on_write.ble_handle_gatt_write(do_handle);
                    }
                }
                _ => {} // ignore other Gatt Connection Events
            }
        }
        Ok(())
    }
    pub fn server(&self) -> &MyServer {
        &self.server
    }
    pub async fn set_and_notify<T: AsGatt + FromGatt>(&self, c: &Characteristic<T>, v: &T) {
        if let Err(e) = c.set(&self.server, v) {
            warn!("{MODULE} Error updating BLE characteristic: {:?}", e);
        }
        if let Some(conn) = self.conn.borrow().as_ref() {
            if let Err(e) = c.notify::<MyPacketPool>(conn, v).await {
                warn!("{MODULE} Error notifying peer: {:?}", e);
            }
        }
    }
    pub fn set<T: AsGatt + FromGatt>(&self, c: &Characteristic<T>, v: &T) {
        if let Err(e) = c.set(&self.server, v) {
            warn!("{MODULE} Error updating BLE characteristic: {:?}", e);
        }
    }
    pub fn get<T: FromGatt>(&self, c: &Characteristic<T>) -> Result<T, Error> {
        Ok(c.get(&self.server)?)
    }
}

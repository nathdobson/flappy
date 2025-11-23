use crate::ble_gatt::{Server, FLAPPY_SERVICE_UUID};
use crate::error::Error;
use core::cell::RefCell;
use core::future::{join, pending};
use core::mem;
use cyw43::bluetooth::BtDriver;
use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_futures::yield_now;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use log::{error, info, warn};
use static_cell::StaticCell;
use trouble_host::advertise::{
    AdStructure, Advertisement, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
};
use trouble_host::attribute::{AttributeTable, Characteristic, Uuid};
use trouble_host::gap::{GapConfig, PeripheralConfig};
use trouble_host::gatt::{GattConnection, GattConnectionEvent, GattEvent};
use trouble_host::prelude::{
    appearance, AsGatt, CccdTable, DefaultPacketPool, ExternalController, FromGatt, Peripheral,
    Runner,
};
use trouble_host::{HostResources, PacketPool, Stack};

/// Max number of connections
const CONNECTIONS_MAX: usize = 1;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 2; // Signal + att

const SLOTS: usize = 10;

pub struct BleModuleBuilder {
    pub spawner: Spawner,
    pub bt_device: BtDriver<'static>,
}

pub struct BleTask {
    peripheral: MyPeripheral,
    server: &'static MyServer,
    stack: &'static MyStack,
    module: &'static BleModule,
}

pub struct BleModule {
    server: &'static MyServer,
    conn: RefCell<Option<MyConnection>>,
}

pub trait BleHandler {
    fn handle_write(&self, id: u16);
}

type MyPacketPool = DefaultPacketPool;
type MyDriver = BtDriver<'static>;
pub type MyController = ExternalController<MyDriver, SLOTS>;
type MyResources = HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>;
pub type MyPeripheral = Peripheral<'static, MyController, MyPacketPool>;
type MyStack = Stack<'static, MyController, MyPacketPool>;
type MyRunner = Runner<'static, MyController, MyPacketPool>;

type MyServer = Server<'static>;

type MyConnection = GattConnection<'static, 'static, MyPacketPool>;

#[embassy_executor::task]
async fn ble_task(mut runner: MyRunner) {
    loop {
        if let Err(e) = runner.run().await {
            panic!("[ble_task] error: {:?}", e);
        }
    }
}

#[embassy_executor::task]
async fn advertise_task(
    mut peripheral: MyPeripheral,
    server: &'static MyServer,
    stack: &'static MyStack,
    module: &'static BleModule,
    handler: &'static dyn BleHandler,
) {
    loop {
        match advertise("Flappy", &mut peripheral, &server).await {
            Ok(conn) => {
                module.conn.borrow_mut().replace(conn);
                match gatt_events_task(
                    &server,
                    module.conn.borrow().as_ref().unwrap(),
                    module,
                    handler,
                )
                .await
                {
                    Ok(_) => {}
                    Err(e) => error!("[gatt_events_task] error: {}", e),
                } //
                module.conn.borrow_mut().take();
            }
            Err(e) => {
                error!("[BLE] error: {}", e);
            }
        }
    }
}

async fn gatt_events_task(
    server: &MyServer,
    conn: &MyConnection,
    inner: &BleModule,
    handler: &'static dyn BleHandler,
) -> Result<(), Error> {
    let reason = loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => break reason,
            GattConnectionEvent::Gatt { event } => {
                let mut do_handle = None;
                match &event {
                    GattEvent::Read(event) => {}
                    GattEvent::Write(event) => {
                        info!("[gatt_events_task] write handle: {:?}", event.handle());
                        info!("[gatt_events_task] write data: {:?}", event.data());
                        do_handle = Some(event.handle());
                    }
                    _ => {}
                };
                // This step is also performed at drop(), but writing it explicitly is necessary
                // in order to ensure reply is sent.
                match event.accept() {
                    Ok(reply) => reply.send().await,
                    Err(e) => warn!("[gatt] error sending response: {:?}", e),
                };
                if let Some(do_handle) = do_handle {
                    handler.handle_write(do_handle);
                }
            }
            _ => {} // ignore other Gatt Connection Events
        }
    };
    info!("[gatt] disconnected: {:?}", reason);
    Ok(())
}

pub async fn advertise(
    name: &'static str,
    peripheral: &mut MyPeripheral,
    server: &'static Server<'static>,
) -> Result<GattConnection<'static, 'static, DefaultPacketPool>, Error> {
    Timer::after_millis(1000).await;
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
            AdStructure::CompleteLocalName(name.as_bytes()),
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
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[BLE] connection established");
    Ok(conn)
}

impl BleModuleBuilder {
    pub async fn build(self) -> Result<(BleTask, &'static BleModule), Error> {
        info!("[BLE] starting");
        yield_now().await;
        let bt = MyController::new(self.bt_device);
        static RESOURCES: StaticCell<MyResources> = StaticCell::new();
        let resources: &'static mut MyResources = RESOURCES.init_with(MyResources::new);
        static STACK: StaticCell<MyStack> = StaticCell::new();
        let stack: &'static MyStack = STACK.init_with(|| trouble_host::new(bt, resources));
        let mut host = stack.build();
        self.spawner.clone().spawn(ble_task(host.runner)?);
        info!("[BLE] beginning advertisement");
        yield_now().await;
        let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
            name: "Flappy",
            appearance: &appearance::domestic_appliance::COFFEE_MAKER,
        }))?;
        static SERVER: StaticCell<Server> = StaticCell::new();
        let server: &'static Server = SERVER.init(server);

        static MODULE: StaticCell<BleModule> = StaticCell::new();
        let module = MODULE.init(BleModule {
            server,
            conn: RefCell::new(None),
        });

        info!("[BLE] started");
        yield_now().await;

        Ok((
            BleTask {
                peripheral: host.peripheral,
                server,
                stack,
                module,
            },
            module,
        ))
    }
}

impl BleTask {
    pub fn spawn(self, spawner: Spawner, handler: &'static dyn BleHandler) -> Result<(), Error> {
        spawner.clone().spawn(advertise_task(
            self.peripheral,
            self.server,
            self.stack,
            self.module,
            handler,
        )?);
        Ok(())
    }
}

impl BleModule {
    pub fn server(&self) -> &MyServer {
        self.server
    }
    pub async fn set_and_notify<T: AsGatt + FromGatt>(&self, c: &Characteristic<T>, v: &T) {
        if let Err(e) = c.set(self.server, v) {
            warn!("Set error: {:?}", e);
        }
        if let Some(conn) = self.conn.borrow().as_ref() {
            if let Err(e) = c.notify::<MyPacketPool>(conn, v).await {
                warn!("Notify error: {:?}", e);
            }
        }
    }
    pub fn set<T: AsGatt + FromGatt>(&self, c: &Characteristic<T>, v: &T) {
        if let Err(e) = c.set(self.server, v) {
            warn!("Set error: {:?}", e);
        }
    }
    pub fn get<T: FromGatt>(&self, c: &Characteristic<T>) -> Result<T, Error> {
        Ok(c.get(self.server)?)
    }
    // pub async fn listen<T: FromGatt>(&self, c: &Characteristic<T>) {
    //     if let Some(signal) = self.writes.get(c.handle as usize) {
    //         signal.wait().await;
    //     } else {
    //         error!("bad handle listened {:?}", c.handle);
    //     }
    // }
}

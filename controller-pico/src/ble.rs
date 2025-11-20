use crate::ble_gatt::{Server, FLAPPY_SERVICE_UUID, HANDLE_LIMIT};
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

struct BleModuleInner {
    conn: RefCell<Option<MyConnection>>,
    writes: [Signal<NoopRawMutex, ()>; HANDLE_LIMIT],
}
pub struct BleModule {
    server: &'static MyServer,
    inner: &'static BleModuleInner,
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
    inner: &'static BleModuleInner,
) {
    loop {
        match advertise("Flappy", &mut peripheral, &server).await {
            Ok(conn) => {
                inner.conn.borrow_mut().replace(conn);
                match gatt_events_task(&server, inner.conn.borrow().as_ref().unwrap(), inner).await
                {
                    Ok(_) => {}
                    Err(e) => error!("[gatt_events_task] error: {}", e),
                } //
                inner.conn.borrow_mut().take();
            }
            Err(e) => {
                error!("[adv] error: {}", e);
            }
        }
    }
}

async fn gatt_events_task(
    server: &MyServer,
    conn: &MyConnection,
    inner: &BleModuleInner,
) -> Result<(), Error> {
    let reason = loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => break reason,
            GattConnectionEvent::Gatt { event } => {
                match &event {
                    GattEvent::Read(event) => {
                        if event.handle() == server.flappy_service.wifi_ssid.handle {
                            info!("[gatt] read wifi ssid");
                        } else if event.handle() == server.flappy_service.wifi_password.handle {
                            info!("[gatt] read wifi password");
                        } else if event.handle() == server.flappy_service.wifi_status.handle {
                            info!("[gatt] read wifi status");
                        } else if event.handle() == server.flappy_service.irc_hostname.handle {
                            info!("[gatt] read wifi irc hostname");
                        } else if event.handle() == server.flappy_service.irc_port.handle {
                            info!("[gatt] read wifi irc port");
                        } else if event.handle() == server.flappy_service.irc_nickname.handle {
                            info!("[gatt] read wifi irc nickname");
                        } else if event.handle() == server.flappy_service.irc_channel.handle {
                            info!("[gatt] read wifi irc channel");
                        } else if event.handle() == server.flappy_service.irc_status.handle {
                            info!("[gatt] read wifi irc status");
                        } else {
                            info!("[gatt] unknown read")
                        }
                    }
                    GattEvent::Write(event) => {
                        if let Some(signal) = inner.writes.get(event.handle() as usize) {
                            signal.signal(());
                        } else {
                            error!("bad id received {:?}", event.handle());
                        }
                        if event.handle() == server.flappy_service.wifi_ssid.handle {
                            info!("[gatt] write wifi ssid");
                        } else if event.handle() == server.flappy_service.wifi_password.handle {
                            info!("[gatt] write wifi password");
                        } else if event.handle() == server.flappy_service.wifi_status.handle {
                            info!("[gatt] write wifi status");
                        } else if event.handle() == server.flappy_service.irc_hostname.handle {
                            info!("[gatt] write wifi irc hostname");
                        } else if event.handle() == server.flappy_service.irc_port.handle {
                            info!("[gatt] write wifi irc port");
                        } else if event.handle() == server.flappy_service.irc_nickname.handle {
                            info!("[gatt] write wifi irc nickname");
                        } else if event.handle() == server.flappy_service.irc_channel.handle {
                            info!("[gatt] write wifi irc channel");
                        } else if event.handle() == server.flappy_service.irc_status.handle {
                            info!("[gatt] write wifi irc status");
                        } else {
                            info!("[gatt] unknown write")
                        }
                    }
                    _ => {}
                };
                // This step is also performed at drop(), but writing it explicitly is necessary
                // in order to ensure reply is sent.
                match event.accept() {
                    Ok(reply) => reply.send().await,
                    Err(e) => warn!("[gatt] error sending response: {:?}", e),
                };
            }
            _ => {} // ignore other Gatt Connection Events
        }
    };
    info!("[gatt] disconnected: {:?}", reason);
    Ok(())
}

pub async fn advertise<'values, 'server>(
    name: &'static str,
    peripheral: &mut MyPeripheral,
    server: &'server Server<'static>,
) -> Result<GattConnection<'static, 'server, DefaultPacketPool>, Error> {
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
    info!("[adv] advertising");
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[adv] connection established");
    Ok(conn)
}

impl BleModuleBuilder {
    pub async fn build(self) -> Result<BleModule, Error> {
        info!("Starting BLE");
        yield_now().await;
        let bt = MyController::new(self.bt_device);
        static RESOURCES: StaticCell<MyResources> = StaticCell::new();
        let resources: &'static mut MyResources = RESOURCES.init_with(MyResources::new);
        static STACK: StaticCell<MyStack> = StaticCell::new();
        let stack: &'static MyStack = STACK.init_with(|| trouble_host::new(bt, resources));
        let mut host = stack.build();

        info!("Starting advertising and GATT service");
        yield_now().await;
        let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
            name: "Flappy",
            appearance: &appearance::domestic_appliance::COFFEE_MAKER,
        }))?;
        static SERVER: StaticCell<Server> = StaticCell::new();
        let server: &'static Server = SERVER.init(server);
        static INNER: StaticCell<BleModuleInner> = StaticCell::new();
        let inner = INNER.init_with(|| BleModuleInner {
            conn: RefCell::new(None),
            writes: [const { Signal::new() }; HANDLE_LIMIT],
        });

        self.spawner.clone().spawn(ble_task(host.runner)?);
        self.spawner
            .clone()
            .spawn(advertise_task(host.peripheral, server, &stack, inner)?);
        info!("Started BLE");
        yield_now().await;

        Ok(BleModule { server, inner })
    }
}

impl BleModule {
    pub fn server(&self) -> &MyServer {
        self.server
    }
    pub async fn set<T: AsGatt + FromGatt>(&self, c: &Characteristic<T>, v: &T) {
        if let Err(e) = c.set(self.server, v) {
            warn!("Set error: {:?}", e);
        }
        if let Some(conn) = self.inner.conn.borrow().as_ref() {
            if let Err(e) = c.notify::<MyPacketPool>(conn, v).await {
                warn!("Notify error: {:?}", e);
            }
        }
    }
    pub fn get<T: FromGatt>(&self, c: &Characteristic<T>) -> Result<T, Error> {
        Ok(c.get(self.server)?)
    }
    pub async fn listen<T: FromGatt>(&self, c: &Characteristic<T>) {
        if let Some(signal) = self.inner.writes.get(c.handle as usize) {
            signal.wait().await;
        } else {
            error!("bad handle listened {:?}", c.handle);
        }
    }
}

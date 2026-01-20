use crate::error::Error;
use crate::product;
use core::cell::{Cell, RefCell};
use core::future::{join, pending};
use core::mem;
use core::str::Utf8Error;
use cyw43::bluetooth::BtDriver;
use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_futures::yield_now;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embassy_sync::channel::{Channel, DynamicReceiver, DynamicSender};
use embassy_sync::signal::Signal;
use embassy_sync::watch::Watch;
use embassy_time::{Duration, Timer, WithTimeout};
use heapless::Vec;
use log::{error, info, warn};
use protocol::ble::SERIAL_MTU;
use protocol::setup::{AppStatus, MAX_SETUP_MESSAGE_SIZE, SetupRequest, SetupResponse};
use protocol::{PRODUCT_NAME, PRODUCT_SHORT_NAME};
use static_cell::make_static;
use trouble_host::advertise::{
    AdStructure, Advertisement, AdvertisementParameters, BR_EDR_NOT_SUPPORTED,
    LE_GENERAL_DISCOVERABLE, PhyKind, TxPower,
};
use trouble_host::attribute::{AttributeTable, Characteristic, Uuid};
use trouble_host::connection::Connection;
use trouble_host::gap::{GapConfig, PeripheralConfig};
use trouble_host::gatt::{GattConnection, GattConnectionEvent, GattEvent};
use trouble_host::l2cap::{CreditFlowPolicy, L2capChannel, L2capChannelConfig};
use trouble_host::prelude::{
    AsGatt, CccdTable, DefaultPacketPool, ExternalController, FromGatt, HeaplessString, Peripheral,
    Runner, appearance, descriptors, gatt_server, gatt_service, uuid,
};
use trouble_host::{HostResources, PacketPool, Stack};

const MODULE: &'static str = "[BLE  ]";
/// Max number of connections
const CONNECTIONS_MAX: usize = 1;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 2;

const SLOTS: usize = 10;
pub const FLAPPY_SERVICE_UUID: Uuid = uuid!("5af0b930-b9b5-11f0-b558-0800200c9a66");

// GATT Server definition
#[gatt_server]
pub struct Server {
    pub flappy_service: FlappyService,
}

/// Battery service
#[gatt_service(uuid = FLAPPY_SERVICE_UUID)]
pub struct FlappyService {
    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "Serial in")]
    #[characteristic(uuid = "4574529b-fbe4-44ae-ba52-d877ac76ef2d", read, notify)]
    pub serial_in: Vec<u8, SERIAL_MTU>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "Serial out")]
    #[characteristic(uuid = "2d2bc907-c9fa-49fd-ba45-410cddf61e5c", write)]
    pub serial_out: Vec<u8, SERIAL_MTU>,

    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, read, value = "App Status")]
    #[characteristic(uuid = "4dc5669d-6bc8-40eb-b6af-8091d4e9b713", read, notify)]
    pub app_status: HeaplessString<256>,
}

pub struct BleModule {
    spawner: Spawner,
    stack: &'static MyStack,
    conn: RefCell<Option<MyConnection>>,
    peri: Cell<Option<MyPeripheral>>,
    server: MyServer,
    setup_request: Channel<NoopRawMutex, SetupRequest, 1>,
    setup_response: Channel<NoopRawMutex, SetupResponse, 1>,
    setup_status: Watch<NoopRawMutex, AppStatus, 1>,
}

type MyPacketPool = DefaultPacketPool;
type MyDriver = BtDriver<'static>;
pub type MyController = ExternalController<MyDriver, SLOTS>;
type MyResources = HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>;
type MyPeripheral = Peripheral<'static, MyController, MyPacketPool>;
type MyStack = Stack<'static, MyController, MyPacketPool>;
type MyRunner = Runner<'static, MyController, MyPacketPool>;

type MyConnection = GattConnection<'static, 'static, MyPacketPool>;
type MyServer = Server<'static>;

impl BleModule {
    pub async fn new(spawner: Spawner, driver: MyDriver) -> Result<&'static BleModule, Error> {
        info!("{MODULE} Starting Bluetooth Low Energy");
        let controller = MyController::new(driver);
        let resources: &mut MyResources = make_static!(MyResources::new());
        let stack: &MyStack = make_static!(trouble_host::new(controller, resources));
        let mut host = stack.build();
        let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
            name: protocol::PRODUCT_NAME,
            appearance: &appearance::domestic_appliance::COFFEE_MAKER,
        }))?;
        let module: &BleModule = make_static!(BleModule {
            spawner,
            stack,
            conn: RefCell::new(None),
            peri: Cell::new(Some(host.peripheral)),
            server,
            setup_request: Channel::new(),
            setup_response: Channel::new(),
            setup_status: Watch::new(),
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
        spawner.clone().spawn({
            #[embassy_executor::task]
            async fn notify_status(module: &'static BleModule) {
                if let Err(e) = module.notify_status().await {
                    error!("{MODULE} error: {:?}", e);
                }
            }
            notify_status(module)?
        });

        info!("{MODULE} Started");

        Ok(module)
    }
    pub fn start(&'static self) -> Result<(), Error> {
        self.spawner.clone().spawn({
            #[embassy_executor::task]
            async fn advertise_task(module: &'static BleModule) {
                module.advertise_loop().await;
            }
            advertise_task(self)?
        });
        Ok(())
    }
    async fn advertise_loop(&'static self) {
        let Some(mut peripheral) = self.peri.take() else {
            return;
        };
        loop {
            match self.advertise(&mut peripheral).await {
                Ok(conn) => {
                    self.conn.borrow_mut().replace(conn);
                    if let Err(e) = self
                        .handle_connection(self.conn.borrow().as_ref().unwrap())
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
    ) -> Result<MyConnection, Error> {
        let mut advertiser_data = [0; 128];
        const FLAPPY_SERVICE_UUID_BYTES: [u8; 16] = {
            match FLAPPY_SERVICE_UUID {
                Uuid::Uuid16(_) => unreachable!(),
                Uuid::Uuid128(x) => x,
            }
        };
        let mut service_uuid = FLAPPY_SERVICE_UUID_BYTES;
        let len = AdStructure::encode_slice(
            &[
                AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                AdStructure::ServiceUuids128(&[service_uuid]),
                AdStructure::CompleteLocalName(PRODUCT_SHORT_NAME.as_bytes()),
            ],
            &mut advertiser_data[..],
        )?;
        info!("{MODULE} advertising");
        let advertiser = peripheral
            .advertise(
                &AdvertisementParameters::default(),
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
    async fn handle_connection(&'static self, conn: &MyConnection) -> Result<(), Error> {
        let mut serial_out_buffer: Vec<u8, MAX_SETUP_MESSAGE_SIZE> = Vec::new();
        let mut serial_in_buffer: Vec<u8, MAX_SETUP_MESSAGE_SIZE>;
        let mut tmp = [0u8; MAX_SETUP_MESSAGE_SIZE];
        loop {
            match conn.next().await {
                GattConnectionEvent::Disconnected { reason } => {
                    break;
                }
                GattConnectionEvent::Gatt { event } => {
                    match &event {
                        GattEvent::Write(event) => {
                            if event.handle() == self.server.flappy_service.serial_out.handle {
                                let new_data =
                                    event.value(&self.server.flappy_service.serial_out)?;
                                serial_out_buffer.extend_from_slice(&new_data)?;
                                if new_data.len() < SERIAL_MTU {
                                    match str::from_utf8(&serial_out_buffer) {
                                        Ok(x) => info!("{MODULE} request {}", x),
                                        Err(e) => {
                                            error!("{MODULE} request {:?}", serial_out_buffer)
                                        }
                                    }
                                    let request = serde_json_core::from_slice_escaped::<
                                        SetupRequest,
                                    >(
                                        &serial_out_buffer, &mut tmp
                                    )?
                                    .0;
                                    serial_out_buffer.clear();
                                    self.setup_request.send(request).await;
                                    let response = self.setup_response.receive().await;
                                    serial_in_buffer = serde_json_core::to_vec(&response)?;
                                    for chunk in serial_in_buffer.chunks(SERIAL_MTU) {
                                        let chunk = Vec::from_slice(chunk)?;
                                        info!("{MODULE} chunk {:?}", chunk);
                                        self.server
                                            .flappy_service
                                            .serial_in
                                            .notify(&conn, &chunk)
                                            .await?;
                                    }
                                    if serial_in_buffer.len() % SERIAL_MTU == 0 {
                                        self.server
                                            .flappy_service
                                            .serial_in
                                            .notify(&conn, &Vec::new())
                                            .await?;
                                    }
                                }
                            }
                        }
                        GattEvent::Read(event) => {}
                        GattEvent::Other(other) => {}
                    };
                    match event.accept() {
                        Ok(reply) => reply.send().await,
                        Err(e) => warn!("{MODULE} error sending response: {:?}", e),
                    };
                }
                _ => {} // ignore other Gatt Connection Events
            }
        }
        Ok(())
    }
    async fn notify_status(&self) -> Result<(), Error> {
        let mut receiver = self
            .setup_status
            .receiver()
            .ok_or(Error::NotEnoughReceivers)?;
        loop {
            let status = receiver.changed().await;
            if let Some(conn) = &*self.conn.borrow() {
                self.server
                    .flappy_service
                    .app_status
                    .notify(&conn, &serde_json_core::to_string(&status)?)
                    .await?;
            }
        }
        Ok(())
    }
    pub fn requests(&'static self) -> DynamicReceiver<'static, SetupRequest> {
        self.setup_request.dyn_receiver()
    }
    pub fn responses(&'static self) -> DynamicSender<'static, SetupResponse> {
        self.setup_response.dyn_sender()
    }
    pub fn update_status(&self, f: impl Fn(&mut AppStatus)) {
        self.setup_status
            .sender()
            .send_modify(move |x| f(x.get_or_insert_default()))
    }
}

use error_report::Report;
use crate::error::Error;
use cyw43::bluetooth::BtDriver;
use embassy_executor::Spawner;
use embassy_executor::raw::TaskPool;
use embassy_rp::clocks::RoscRng;
use log::{error, info};
use static_cell::StaticCell;
use trouble_host::advertise::{
    AdStructure, Advertisement, AdvertisementParameters, BR_EDR_NOT_SUPPORTED,
    LE_GENERAL_DISCOVERABLE,
};
use trouble_host::attribute::Uuid;
use trouble_host::connection::Connection;
use trouble_host::gatt::GattConnection;
use trouble_host::peripheral::Peripheral;
use trouble_host::prelude::{DefaultPacketPool, ExternalController, Runner};
use trouble_host::{Address, HostResources, IoCapabilities, Stack};

pub struct BlePeripherals {
    pub(crate) ble: cyw43::bluetooth::BtDriver<'static>,
    pub(crate) mac_address: [u8; 6],
}

pub struct BleStack<
    const SLOTS: usize,
    const CONNS: usize,
    const CHANNELS: usize,
    const ADV_SETS: usize = 1,
    const BONDS: usize = 10,
> {
    resources: StaticCell<MyResources<SLOTS, CONNS, CHANNELS, ADV_SETS, BONDS>>,
    stack: StaticCell<MyStack<SLOTS>>,
    runner: StaticCell<TaskPool<MyRunnerWrapper<SLOTS>, 1>>,
}

pub struct BleBuilder<
    const SLOTS: usize,
    const CONNS: usize,
    const CHANNELS: usize,
    const ADV_SETS: usize = 1,
    const BONDS: usize = 10,
> {
    pub peripherals: BlePeripherals,
    pub spawner: Spawner,
    pub stack: &'static BleStack<SLOTS, CONNS, CHANNELS, ADV_SETS, BONDS>,
}

type MyPacketPool = DefaultPacketPool;
type MyDriver = BtDriver<'static>;
pub type MyController<const SLOTS: usize> = ExternalController<MyDriver, SLOTS>;
type MyResources<
    const SLOTS: usize,
    const CONNS: usize,
    const CHANNELS: usize,
    const ADV_SETS: usize,
    const BONDS: usize,
> = HostResources<MyController<SLOTS>, MyPacketPool, CONNS, CHANNELS, ADV_SETS, BONDS>;
pub(crate) type MyPeripheral<const SLOTS: usize> =
    Peripheral<'static, MyController<SLOTS>, MyPacketPool>;
type MyStack<const SLOTS: usize> = Stack<'static, MyController<SLOTS>, MyPacketPool>;
type MyRunner<const SLOTS: usize> = Runner<'static, MyController<SLOTS>, MyPacketPool>;

type MyRunnerWrapper<const SLOTS: usize> = impl Future<Output = ()> + 'static;
type MyConnection = Connection<'static, MyPacketPool>;
pub(crate) type MyGattConnection = GattConnection<'static, 'static, MyPacketPool>;

#[define_opaque(MyRunnerWrapper)]
fn my_runner_wrapper<const SLOTS: usize>(mut runner: MyRunner<SLOTS>) -> MyRunnerWrapper<SLOTS> {
    async move {
        if let Err(e) = runner.run().await {
            error!("BLE system error: {}", Report::new(e));
        }
    }
}

impl<
    const SLOTS: usize,
    const CONNS: usize,
    const CHANNELS: usize,
    const ADV_SETS: usize,
    const BONDS: usize,
> BleStack<SLOTS, CONNS, CHANNELS, ADV_SETS, BONDS>
{
    pub fn new() -> Self {
        BleStack {
            resources: StaticCell::new(),
            stack: StaticCell::new(),
            runner: StaticCell::new(),
        }
    }
}

impl<
    const SLOTS: usize,
    const CONNS: usize,
    const CHANNELS: usize,
    const ADV_SETS: usize,
    const BONDS: usize,
> BleBuilder<SLOTS, CONNS, CHANNELS, ADV_SETS, BONDS>
{
    pub fn build(self) -> Result<MyPeripheral<SLOTS>, Error> {
        info!("Starting Bluetooth Low Energy");
        let controller = MyController::new(self.peripherals.ble);
        let resources = self.stack.resources.init_with(MyResources::new);
        let stack = self.stack.stack.init_with(|| {
            trouble_host::new(controller, resources)
                .set_random_address(Address::random(self.peripherals.mac_address))
                .set_random_generator_seed(&mut RoscRng)
                .set_secure_connections_only(true)
                .set_io_capabilities(IoCapabilities::NoInputNoOutput)
                .build()
        });
        let _central = stack.central();
        let runner = stack.runner();
        let peri = stack.peripheral();

        self.spawner.spawn(
            self.stack
                .runner
                .init_with(TaskPool::new)
                .spawn(|| my_runner_wrapper::<SLOTS>(runner))?,
        );
        info!("BLE Started");

        Ok(peri)
    }
}

pub struct AdvertiseBuilder<'a, const SLOTS: usize> {
    pub peri: &'a mut MyPeripheral<SLOTS>,
    pub service: Uuid,
    pub short_name: &'static str,
}

impl<'a, const SLOTS: usize> AdvertiseBuilder<'a, SLOTS> {
    pub async fn build(self) -> Result<MyConnection, Error> {
        let mut advertiser_data = [0; 128];
        let uuid16;
        let uuid32;
        let uuid128;
        let service = match self.service {
            Uuid::Uuid16(x) => {
                uuid16 = [x];
                AdStructure::IncompleteServiceUuids16(&uuid16)
            }
            Uuid::Uuid32(x) => {
                uuid32 = [x];
                AdStructure::IncompleteServiceUuids32(&uuid32)
            }
            Uuid::Uuid128(x) => {
                uuid128 = [x];
                AdStructure::IncompleteServiceUuids128(&uuid128)
            }
        };
        let len = AdStructure::encode_slice(
            &[
                AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                service,
                AdStructure::CompleteLocalName(self.short_name.as_bytes()),
            ],
            &mut advertiser_data[..],
        )?;
        info!("BLE advertising");
        let advertiser = self
            .peri
            .advertise(
                &AdvertisementParameters::default(),
                Advertisement::ConnectableScannableUndirected {
                    adv_data: &advertiser_data[..len],
                    scan_data: &[],
                },
            )
            .await?;
        let connection = advertiser.accept().await?;
        Ok(connection)
    }
}

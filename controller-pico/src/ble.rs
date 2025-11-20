use crate::error::Error;
use crate::ble_gatt::{advertise, custom_task, gatt_events_task, Server};
use core::future::join;
use cyw43::bluetooth::BtDriver;
use embassy_executor::Spawner;
use embassy_futures::select::select;
use log::{error, info};
use trouble_host::gap::{GapConfig, PeripheralConfig};
use trouble_host::prelude::{
    appearance, DefaultPacketPool, ExternalController, Peripheral, Runner,
};
use trouble_host::{HostResources, Stack};

/// Max number of connections
const CONNECTIONS_MAX: usize = 1;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 2; // Signal + att

const SLOTS: usize = 10;

pub struct BleModuleBuilder {
    pub spawner: Spawner,
    pub bt_device: BtDriver<'static>,
}

pub struct BleModule {}

type MyPacketPool = DefaultPacketPool;
type MyDriver = BtDriver<'static>;
pub type MyController = ExternalController<MyDriver, SLOTS>;
type MyResources = HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>;
pub type MyPeripheral<'a> = Peripheral<'a, MyController, MyPacketPool>;
type MyStack<'a> = Stack<'a, MyController, MyPacketPool>;

async fn ble_task<'a>(mut runner: Runner<'a, MyController, MyPacketPool>) {
    loop {
        if let Err(e) = runner.run().await {
            panic!("[ble_task] error: {:?}", e);
        }
    }
}

async fn advertise_task<'a>(
    mut peripheral: MyPeripheral<'a>,
    server: Server<'a>,
    stack: &'a MyStack<'a>,
) {
    loop {
        match advertise("Flappy", &mut peripheral, &server).await {
            Ok(conn) => {
                let a = gatt_events_task(&server, &conn);
                let b = custom_task(&server, &conn, stack);
                select(a, b).await;
            }
            Err(e) => {
                panic!("[adv] error: {:?}", e);
            }
        }
    }
}

#[embassy_executor::task]
async fn build_task(builder: BleModuleBuilder) {
    let bt = MyController::new(builder.bt_device);
    let mut resources = MyResources::new();
    let stack = trouble_host::new(bt, &mut resources);
    let mut host = stack.build();

    info!("Starting advertising and GATT service");
    let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "Flappy",
        appearance: &appearance::domestic_appliance::COFFEE_MAKER,
    }));
    let server = match server {
        Ok(server) => server,
        Err(error) => {
            error!("[ble] error when constructing server: {:?}", error);
            return;
        }
    };

    join!(
        ble_task(host.runner),
        advertise_task(host.peripheral, server, &stack)
    )
    .await;
}

impl BleModuleBuilder {
    #[must_use]
    pub async fn build(self) -> Result<BleModule, Error> {
        self.spawner.clone().spawn(build_task(self)?);

        Ok(BleModule {})
    }
}

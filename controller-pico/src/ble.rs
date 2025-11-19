use cyw43::bluetooth::BtDriver;
use log::info;
use trouble_host::prelude::ExternalController;

pub struct BleModuleBuilder {
    pub bt_device: BtDriver<'static>,
}

pub struct BleModule {}

impl BleModuleBuilder {
    #[must_use]
    pub async fn build(self) -> BleModule {
        let bt = ExternalController::<_, 10>::new(self.bt_device);
        use trouble_example_apps::ble_bas_peripheral;
        info!("a");
        ble_bas_peripheral::run(bt).await;
        info!("b");
        BleModule {}
    }
}

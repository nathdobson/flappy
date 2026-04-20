use crate::radio::RadioModule;
use core::cell::RefCell;
use embassy_executor::{SpawnError, Spawner};
use embassy_futures::yield_now;
use embassy_time::{Duration, Timer};
use log::info;
use make_static::make_static;

const MODULE: &str = "[LED  ]";
const INCREMENTS: u64 = 20;
const DELAY_NANOS: f32 = 10_000_000f32;

pub struct LedModule {
    radio: &'static RadioModule,
}

impl LedModule {
    pub async fn new(
        spawner: Spawner,
        radio: &'static RadioModule,
    ) -> Result<&'static LedModule, SpawnError> {
        let delay = Duration::from_millis(250);
        info!("{MODULE} starting");
        let module = make_static!(LedModule, LedModule { radio });
        spawner.spawn({
            #[embassy_executor::task]
            async fn blink_task(module: &'static LedModule) {
                module.blink().await;
            }
            blink_task(module)?
        });
        info!("{MODULE} started");
        Ok(module)
    }
    async fn blink(&self) {
        for i in 0u64.. {
            let intensity = ((1.0 + core::intrinsics::sinf32((i as f32) / (INCREMENTS as f32)))
                / 2.0)
                .clamp(0.0, 1.0);
            let duty = core::intrinsics::powf32(intensity, 2.0);
            let wait1 = (DELAY_NANOS * duty) as u64;
            let wait2 = (DELAY_NANOS * (1.0 - duty)) as u64;
            let wait1 = Duration::from_nanos(wait1);
            let wait2 = Duration::from_nanos(wait2);
            self.radio.control.lock().await.gpio_set(0, true).await;
            Timer::after(wait1).await;
            self.radio.control.lock().await.gpio_set(0, false).await;
            Timer::after(wait2).await;
        }
    }
}

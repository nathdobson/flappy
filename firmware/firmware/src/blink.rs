// use crate::radio::RadioModule;
use core::cell::RefCell;
use embassy_executor::{SpawnError, Spawner};
use embassy_futures::yield_now;
use embassy_time::{Duration, Timer};
use log::info;
use make_static::make_static;
use radio_builder::Radio;
use radio_builder::led::RadioLed;
use runtime::LocalSpawn;

const MODULE: &str = "[LED  ]";
const INCREMENTS: u64 = 20;
const DELAY_NANOS: f32 = 10_000_000f32;

pub struct BlinkModule {
    led: &'static RadioLed,
}

impl BlinkModule {
    pub fn new(
        spawner: Spawner,
        led: &'static RadioLed,
    ) -> Result<&'static BlinkModule, SpawnError> {
        let delay = Duration::from_millis(250);
        info!("{MODULE} starting");
        let module: &_ = make_static!(BlinkModule, BlinkModule { led });
        make_static!(_, LocalSpawn::new(spawner)).spawn(move || async move {
            module.blink().await;
        });
        info!("{MODULE} started");
        Ok(module)
    }
    async fn blink(&self) {
        for i in 0u64.. {
            let intensity = ((1.0 + core::intrinsics::sin((i as f32) / (INCREMENTS as f32)))
                / 2.0)
                .clamp(0.0, 1.0);
            let duty = core::intrinsics::powf32(intensity, 2.0);
            let wait1 = (DELAY_NANOS * duty) as u64;
            let wait2 = (DELAY_NANOS * (1.0 - duty)) as u64;
            let wait1 = Duration::from_nanos(wait1);
            let wait2 = Duration::from_nanos(wait2);
            self.led.set_led(true).await;
            Timer::after(wait1).await;
            self.led.set_led(false).await;
            Timer::after(wait2).await;
        }
    }
}

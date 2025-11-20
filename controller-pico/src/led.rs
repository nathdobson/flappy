use crate::error::Result;
use cyw43::Control;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use log::info;
const INCREMENTS: u64 = 20;
const DELAY_NANOS: f32 = 10000000f32;

#[embassy_executor::task]
async fn led_task(mut control: Control<'static>) {
    let total = INCREMENTS;
    for i in 0u64.. {
        let intensity =
            ((1.0 + unsafe { core::intrinsics::sinf32((i as f32) / (INCREMENTS as f32)) }) / 2.0)
                .clamp(0.0, 1.0);
        let duty = unsafe { core::intrinsics::powf32(intensity, 2.0) };
        let wait1 = (DELAY_NANOS * duty) as u64;
        let wait2 = (DELAY_NANOS * (1.0 - duty)) as u64;
        let wait1 = Duration::from_nanos(wait1);
        let wait2 = Duration::from_nanos(wait2);
        control.gpio_set(0, true).await;
        Timer::after(wait1).await;
        control.gpio_set(0, false).await;
        Timer::after(wait2).await;
    }
}

pub struct LedModuleBuilder {
    pub spawner: Spawner,
    pub control: Control<'static>,
}

pub struct LedModule {}

impl LedModuleBuilder {
    pub fn build(self) -> Result<LedModule> {
        self.spawner.spawn(led_task(self.control)?);
        Ok(LedModule {})
    }
}

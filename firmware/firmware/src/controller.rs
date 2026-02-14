use crate::driver::DriverModule;
use crate::error::Error;
use crate::{display_proto, make_static};
use core::cell::RefCell;
use core::default::Default;
use core::ops::Index;
use core::{fmt, iter};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::{Mutex, MutexGuard};
use embassy_time::Timer;
use heapless::{String, Vec};
use log::{error, info};
use protocol::display::MAX_GLYPHS;
use protocol::setup::{DisplaySettings, DriverVersion};

const MODULE: &'static str = "[CTRL ]";
const STEPS_PER_REV: usize = 2048;
const FLAP_COUNT: usize = 45;
#[derive(Debug)]
pub struct SegmentController {
    target: isize,
    position: isize,

    prev_hall: Option<bool>,
    homed: bool,

    phase: usize,
    spinning: bool,
}

pub struct DisplayController {
    driver: &'static DriverModule,
    segments: Vec<SegmentController, MAX_GLYPHS>,
}

pub struct DisplayControllerGuard<'a> {
    settings: DisplaySettings,
    guard: MutexGuard<'a, NoopRawMutex, DisplayController>,
}

pub struct ControllerModule {
    display: Mutex<NoopRawMutex, DisplayController>,
    settings: RefCell<DisplaySettings>,
}

impl ControllerModule {
    pub fn set_settings(&self, settings: DisplaySettings) {
        *self.settings.borrow_mut() = settings;
    }
}

impl<'a> Drop for DisplayControllerGuard<'a> {
    fn drop(&mut self) {
        for segment in &mut self.guard.segments {
            segment.spinning = false;
        }
        self.guard.driver.set_enabled(false);
    }
}

impl ControllerModule {
    pub fn new(driver: &'static DriverModule) -> &'static Self {
        make_static!(
            ControllerModule,
            ControllerModule {
                display: Mutex::new(DisplayController {
                    driver,
                    segments: Vec::new()
                }),
                settings: RefCell::new(DisplaySettings::default()),
            }
        )
    }
    async fn enable(&self) -> DisplayControllerGuard<'_> {
        let mut guard = self.display.lock().await;
        guard.driver.set_enabled(true);
        DisplayControllerGuard {
            settings: self.settings.borrow().clone(),
            guard,
        }
    }
    pub async fn run(&self, message: &[usize]) -> Result<(), Error> {
        Ok(self.enable().await.run(message).await?)
    }
}

impl SegmentController {
    fn advance(&mut self) {
        self.position += 1;
        self.phase = (self.phase + 1) % 4;
    }
}

impl<'a> DisplayControllerGuard<'a> {
    async fn run(&mut self, message: &[usize]) -> Result<(), Error> {
        self.recount()?;
        self.set_targets(message);
        self.drive_all_to_targets(message).await?;
        Ok(())
    }
    fn recount(&mut self) -> Result<(), Error> {
        let count = self.guard.driver.count()?;
        if count > MAX_GLYPHS {
            return Err("Too many characters in series".into());
        }
        while self.guard.segments.len() > count {
            self.guard.segments.pop();
        }
        while self.guard.segments.len() < count {
            self.guard
                .segments
                .push(SegmentController {
                    target: 0,
                    position: 0,
                    prev_hall: None,
                    homed: false,
                    phase: 0,
                    spinning: false,
                })
                .ok()
                .unwrap();
        }
        Ok(())
    }
    fn set_targets(&mut self, message: &[usize]) {
        for (index, char) in self.guard.segments.iter_mut().enumerate() {
            let calibration = self.settings.calibration.get(index).cloned().unwrap_or(0);
            let new_target = ((calibration
                + message.get(index).cloned().unwrap_or(0) * STEPS_PER_REV / FLAP_COUNT)
                % STEPS_PER_REV) as isize;
            if !char.homed || new_target != char.position {
                char.target = new_target;
                char.spinning = true;
            }
        }
    }
    fn write_to_drivers(&mut self) -> Result<(), Error> {
        let reverse = match self.settings.driver_version {
            DriverVersion::V1_0 => true,
            DriverVersion::V2_0 => false,
        };
        let mut output_buffer = Vec::<u8, { (MAX_GLYPHS + 1) / 2 }>::new();
        for cs in self.guard.segments.chunks_mut(2) {
            let mut b = 0;
            for (i, c) in cs.iter_mut().enumerate() {
                let mut mask = 0;

                if c.spinning {
                    let phase1 = if reverse { 3 - c.phase } else { c.phase };
                    // full step drive (two phases enabled at a time)
                    let phase2 = (phase1 + 1) % 4;
                    mask = (1 << phase1) | (1 << phase2);
                }
                // encode two motors per byte
                b |= mask << i * 4;
            }
            output_buffer.push(b).unwrap();
        }
        output_buffer.reverse();
        self.guard.driver.write(&output_buffer)?;
        Ok(())
    }
    fn read_from_drivers(&mut self) -> Result<(), Error> {
        let mut input_buffer = Vec::<u8, { MAX_GLYPHS + 1 }>::new();
        input_buffer
            .resize(self.guard.segments.len() + 1, 0)
            .unwrap();
        self.guard.driver.read(&mut input_buffer)?;
        if input_buffer[self.guard.segments.len()] != 0xFF {
            error!(
                "{MODULE} Hall sensor read error (Bad terminator {})",
                input_buffer[self.guard.segments.len()]
            );
        }
        for i in 0..self.guard.segments.len() {
            let fault;
            let hall;
            let zeros;
            match self.settings.driver_version {
                DriverVersion::V1_0 => {
                    fault = input_buffer[i] & 1 != 1;
                    hall = input_buffer[i] & 2 == 2;
                    zeros = input_buffer[i] & !0b11;
                }
                DriverVersion::V2_0 => {
                    fault = false;
                    hall = input_buffer[i] & 1 == 1;
                    zeros = input_buffer[i] & !0b1;
                }
            }
            if zeros != 0 {
                error!(
                    "{MODULE} Hall sensor read error (bad bits {}: {})",
                    i, zeros
                );
            }
            if fault {
                error!("{MODULE} Motor fault {}", i);
            }
            if Some(hall) != self.guard.segments[i].prev_hall {
                if Some(false) == self.guard.segments[i].prev_hall {
                    self.guard.segments[i].homed = true;
                    info!(
                        "{MODULE} Flap {} homed at {:?}",
                        i, self.guard.segments[i].position
                    );
                    self.guard.segments[i].position = 0;
                }
                self.guard.segments[i].prev_hall = Some(hall);
            }
        }
        Ok(())
    }
    async fn drive_all_to_targets(&mut self, message: &[usize]) -> Result<(), Error> {
        let delay_micros = self.settings.delay_micros.unwrap_or(3000);
        let delay_micros_init = self.settings.delay_micros_init.unwrap_or(10000);
        let delay_accel_steps = self.settings.delay_accel_steps.unwrap_or(128);
        for step in 0u64.. {
            let delay = if step < delay_accel_steps {
                let speed_init = 1.0f64 / (delay_micros_init as f64);
                let speed = 1.0f64 / (delay_micros as f64);
                let average = (speed_init * ((delay_accel_steps - step) as f64)
                    + speed * (step as f64))
                    / (delay_accel_steps as f64);
                (1.0 / average) as u64
            } else {
                delay_micros
            };
            let timer = Timer::after_micros(delay);
            let mut done = true;
            for char in &mut self.guard.segments {
                if char.spinning {
                    if char.position == char.target && char.homed {
                        done = false;
                        char.spinning = false;
                    } else if char.position > ((STEPS_PER_REV * 3) / 2) as isize {
                        // We're well past the homing point, so the homing sensor must be malfunctioning.
                        char.spinning = false;
                        char.homed = false;
                        char.position = 0;
                        char.prev_hall = None;
                        char.target = 0;
                        continue;
                    } else {
                        done = false;
                        char.advance();
                    }
                }
            }
            self.write_to_drivers()?;
            self.read_from_drivers()?;
            if done {
                break;
            }
            timer.await;
        }
        for (index, char) in self.guard.segments.iter().enumerate() {
            if !char.homed {
                error!("Failed to home {}", index);
            }
        }
        Ok(())
    }
}

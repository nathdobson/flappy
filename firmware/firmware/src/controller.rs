use crate::driver::DriverModule;
use crate::error::Error;
use core::cell::RefCell;
use core::default::Default;
use core::ops::Index;
use core::{fmt, iter};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::{Mutex, MutexGuard};
use embassy_time::{Instant, Timer};
use heapless::{CapacityError, String, Vec};
use log::{error, info};
use make_static::make_static;
use protocol::display::{MAX_GLYPHS, STEPS_PER_REVOLUTION};
use protocol::setup::{DisplaySettings, DriverVersion};

const MODULE: &'static str = "[CTRL ]";
const FLAP_COUNT: usize = 45;
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum SegmentMode {
    Disabled,
    Spinning,
    Decelerating,
    Holding(usize),
}
#[derive(Debug)]
pub struct SegmentController {
    target: isize,
    position: isize,

    prev_hall: Option<bool>,
    homed: bool,

    phase: usize,
    mode: SegmentMode,
    ticks_until_step: usize,
    accel_step: usize,
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
            segment.mode = SegmentMode::Disabled;
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

struct AccelerationCurve {
    micros_per_tick: u64,
    ticks_per_step: Vec<u8, 1024>,
}

struct AccelerationStage {
    ticks_per_step: u32,
    steps_per_stage: u32,
}

impl AccelerationCurve {
    pub fn new(settings: &DisplaySettings) -> Result<AccelerationCurve, Error> {
        let micros_per_tick = settings.micros_per_tick.unwrap_or(350);
        let slow_ticks_per_step = settings.slow_ticks_per_step.unwrap_or(3);
        let fast_ticks_per_step = settings.fast_ticks_per_step.unwrap_or(6);
        let slow_steps_per_stage = settings.slow_steps_per_stage.unwrap_or(6);
        let mut ticks_per_step_vec = Vec::new();
        let slow_ticks_per_step_1 = slow_ticks_per_step as f32;
        let slow_ticks_per_step_2 = slow_ticks_per_step_1 * slow_ticks_per_step_1;
        let slow_ticks_per_step_4 = slow_ticks_per_step_2 * slow_ticks_per_step_2;
        for ticks_per_step in (fast_ticks_per_step + 1..=slow_ticks_per_step).rev() {
            // This formula delivers approximately constant mechanical power.
            let ticks_per_step_1 = ticks_per_step as f32;
            let ticks_per_step_2 = ticks_per_step_1 * ticks_per_step_1;
            let ticks_per_step_4 = ticks_per_step_2 * ticks_per_step_2;
            let steps_per_stage =
                ((slow_steps_per_stage as f32) * slow_ticks_per_step_4 / ticks_per_step_4) as u32;
            for i in 0..steps_per_stage {
                ticks_per_step_vec
                    .push(ticks_per_step)
                    .map_err(|_| Error::CapacityError)?;
            }
        }
        ticks_per_step_vec
            .push(fast_ticks_per_step)
            .map_err(|_| Error::CapacityError)?;
        Ok(AccelerationCurve {
            micros_per_tick,
            ticks_per_step: ticks_per_step_vec,
        })
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
            return Err(Error::CapacityError);
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
                    mode: SegmentMode::Disabled,
                    ticks_until_step: 0,
                    accel_step: 0,
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
                + message.get(index).cloned().unwrap_or(0) * STEPS_PER_REVOLUTION / FLAP_COUNT)
                % STEPS_PER_REVOLUTION) as isize;
            if !char.homed || new_target != char.position {
                char.target = new_target;
                char.mode = SegmentMode::Spinning;
                char.accel_step = 0;
                if self.settings.rehome_after_stopping {
                    char.homed = false;
                    char.position = 0;
                }
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

                if c.mode != SegmentMode::Disabled {
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
        let curve = AccelerationCurve::new(&self.settings)?;
        for step in 0u64.. {
            let timer = Timer::after_micros(curve.micros_per_tick);
            let mut done = true;
            for char in &mut self.guard.segments {
                match &mut char.mode {
                    SegmentMode::Disabled => {
                        continue;
                    }
                    _ => {}
                }
                if char.position > ((STEPS_PER_REVOLUTION * 3) / 2) as isize {
                    // We're well past the homing point, so the homing sensor must be malfunctioning.
                    char.mode = SegmentMode::Disabled;
                    char.homed = false;
                    char.position = 0;
                    char.prev_hall = None;
                    char.target = 0;
                    continue;
                }
                done = false;
                if char.ticks_until_step <= 1 {
                    match &mut char.mode {
                        SegmentMode::Disabled => unreachable!(),
                        SegmentMode::Spinning => {
                            char.advance();
                            char.ticks_until_step = curve.ticks_per_step[char.accel_step] as usize;
                            if char.accel_step < curve.ticks_per_step.len() - 1 {
                                char.accel_step += 1;
                            }
                            if char.homed {
                                let distance = (char.target as usize + STEPS_PER_REVOLUTION
                                    - char.position as usize)
                                    % STEPS_PER_REVOLUTION;
                                if char.accel_step <= distance && distance <= char.accel_step + 1 {
                                    char.mode = SegmentMode::Decelerating;
                                }
                            }
                        }
                        SegmentMode::Decelerating => {
                            char.advance();
                            char.ticks_until_step = curve.ticks_per_step[char.accel_step] as usize;
                            if char.accel_step > 0 {
                                char.accel_step -= 1;
                            }
                            if char.position as usize % STEPS_PER_REVOLUTION == char.target as usize
                            {
                                char.mode = SegmentMode::Holding(10);
                            }
                        }
                        SegmentMode::Holding(countdown) => {
                            if *countdown == 0 {
                                char.mode = SegmentMode::Disabled;
                            } else {
                                *countdown -= 1;
                            }
                        }
                    }
                } else {
                    char.ticks_until_step -= 1;
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

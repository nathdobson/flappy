use crate::driver::DriverModule;
use crate::error::Error;
use core::cell::RefCell;
use core::default::Default;
use core::ops::Index;
use core::{fmt, iter};
use embassy_time::Timer;
use heapless::{String, Vec};
use log::{error, info};
use crate::display_proto;
use crate::display_proto::DisplaySettings;
use proto::MAX_GLYPHS;

const MODULE: &'static str = "[DISPL]";
const STEPS_PER_REV: usize = 2048;
const FLAP_COUNT: usize = 45;

pub struct DisplayCharacter {
    target: usize,
    position: usize,

    prev_hall: Option<bool>,
    homed: bool,

    phase: usize,
    charged: bool,
}

pub struct Display {
    driver: &'static DriverModule,
    glyphs: Vec<DisplayCharacter, MAX_GLYPHS>,
    settings: RefCell<DisplaySettings>,
}

impl Display {
    pub fn set_settings(&self, settings: DisplaySettings) {
        *self.settings.borrow_mut() = settings;
    }
}

impl Display {
    pub fn new(driver: &'static DriverModule) -> Self {
        Display {
            driver,
            glyphs: Vec::new(),
            settings: RefCell::new(DisplaySettings::default()),
        }
    }
    pub async fn run(&mut self, message: &[usize]) -> Result<(), Error> {
        self.driver.set_enabled(true);
        let count = self.driver.count()?;
        if count > MAX_GLYPHS {
            return Err("Too many characters in series".into());
        }
        while self.glyphs.len() > count {
            self.glyphs.pop();
        }
        while self.glyphs.len() < count {
            self.glyphs
                .push(DisplayCharacter {
                    target: 0,
                    position: 0,
                    prev_hall: None,
                    homed: false,
                    phase: 0,
                    charged: false,
                })
                .ok()
                .unwrap();
        }
        for char in self.glyphs.iter_mut() {
            char.position = 0;
            char.homed = false;
            char.prev_hall = None;
            char.charged = true;
        }
        for (index, char) in self.glyphs.iter_mut().enumerate() {
            let calibration = self
                .settings
                .borrow()
                .calibration
                .get(index)
                .cloned()
                .unwrap_or(0);
            char.target =
                (calibration + message.get(index).cloned().unwrap_or(0) * STEPS_PER_REV / FLAP_COUNT) % STEPS_PER_REV;
            info!("{MODULE} Flap {index} has target {:?}", char.target);
        }
        for step in 0.. {
            let timer = Timer::after_micros(3000);
            let mut done = true;
            for char in &mut self.glyphs {
                if !char.charged {
                    continue;
                }
                if char.position == char.target && char.homed {
                    char.charged = false;
                    continue;
                }
                // We're well past the homing point, so the homing sensor must be malfunctioning.
                if char.position > (STEPS_PER_REV * 3) / 2 {
                    char.charged = false;
                    continue;
                }
                done = false;
                char.position = char.position + 1;
                char.phase = (char.phase + 1) % 4;
            }
            let mut output_buffer = Vec::<u8, { (MAX_GLYPHS + 1) / 2 }>::new();
            for cs in self.glyphs.chunks_mut(2) {
                let mut b = 0;
                for (i, c) in cs.iter_mut().enumerate() {
                    let mut mask = 0;
                    if c.charged {
                        // Run the motor in reverse
                        let phase1 = 3 - c.phase;
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
            self.driver.write(&output_buffer)?;
            let mut input_buffer = Vec::<u8, { MAX_GLYPHS + 1 }>::new();
            input_buffer.resize(count + 1, 0).unwrap();
            self.driver.read(&mut input_buffer)?;
            if input_buffer[count] != 0xFF {
                error!(
                    "{MODULE} Hall sensor read error (Bad terminator {})",
                    input_buffer[count]
                );
            }
            for i in 0..count {
                let fault = input_buffer[i] & 1 == 1;
                let hall = input_buffer[i] & 2 == 2;
                let zeros = input_buffer[i] & !0b11;
                if zeros != 0 {
                    error!(
                        "{MODULE} Hall sensor read error (bad bits {}: {})",
                        i, zeros
                    );
                }
                if !fault {
                    error!("{MODULE} Motor fault {}", i);
                }
                if Some(hall) != self.glyphs[i].prev_hall {
                    if let Some(prev_hall) = self.glyphs[i].prev_hall {
                        if !prev_hall {
                            self.glyphs[i].homed = true;
                            info!("{MODULE} Flap {} homed at {:?}", i, self.glyphs[i].position);
                            self.glyphs[i].position = 0;
                        }
                    }
                    self.glyphs[i].prev_hall = Some(hall);
                }
            }
            if done {
                break;
            }
            timer.await;
        }
        self.driver.set_enabled(false);
        for (index, char) in self.glyphs.iter().enumerate() {
            if !char.homed {
                error!("Failed to home {}", index);
            }
        }
        Ok(())
    }
}

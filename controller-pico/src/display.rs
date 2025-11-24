use crate::driver::DriverModule;
use crate::error::Error;
use core::default::Default;
use core::{fmt, iter};
use embassy_time::Timer;
use heapless::Vec;
use log::info;
use unicode_segmentation::UnicodeSegmentation;

const MAX_CHARS: usize = 12;
const STEPS_PER_REV: usize = 2048;
const FLAP_COUNT: usize = 45;

pub struct DisplayCharacter {
    target: Option<usize>,
    phase: usize,
    prev_hall: Option<bool>,
    homed: bool,
    position: Option<usize>,
    calibration: usize,
}

pub struct Display {
    driver: &'static DriverModule,
    chars: Vec<DisplayCharacter, MAX_CHARS>,
}

impl Display {
    pub fn new(driver: &'static DriverModule) -> Self {
        Display {
            driver,
            chars: Vec::new(),
        }
    }
    pub async fn run(&mut self, message: &str) -> Result<(), Error> {
        self.driver.set_enabled(true);
        let count = self.driver.count()?.min(MAX_CHARS);
        while self.chars.len() > count {
            self.chars.pop();
        }
        while self.chars.len() < count {
            self.chars
                .push(DisplayCharacter {
                    target: None,
                    phase: 0,
                    prev_hall: None,
                    homed: false,
                    position: None,
                    calibration: [1858, 1848, 1848, 1868, 1868][self.chars.len()],
                })
                .ok()
                .unwrap();
        }
        for (index, char) in UnicodeSegmentation::graphemes(message, true)
            .chain(iter::repeat(" "))
            .enumerate()
            .take(count)
        {
            let flap = char
                .chars()
                .filter_map(|c| common::LETTERS.find(c.to_ascii_uppercase()))
                .next()
                .unwrap_or(0);
            self.chars[index].target = Some(
                (self.chars[index].calibration + flap * STEPS_PER_REV / FLAP_COUNT) % STEPS_PER_REV,
            );
            info!("Target = {:?}", self.chars[index].target);
        }
        for step in 0.. {
            Timer::after_micros(2000).await;
            let mut done = true;
            for char in &mut self.chars {
                if let Some(target) = char.target {
                    if let Some(position) = &mut char.position {
                        if *position == target {
                            continue;
                        } else {
                            *position = (*position + 1) % STEPS_PER_REV;
                        }
                    }
                }
                done = false;
                char.phase = (char.phase + 1) % 4;
            }
            let mut output_buffer = Vec::<u8, { MAX_CHARS / 2 }>::new();
            for cs in self.chars.chunks_mut(2) {
                let mut b = 0;
                for (i, c) in cs.iter_mut().enumerate() {
                    // Run the motor in reverse
                    let phase1 = 3 - c.phase;
                    // full step drive (two phases enabled at a time)
                    let phase2 = (phase1 + 1) % 4;
                    let mask = (1 << phase1) | (1 << phase2);
                    // encode two motors per byte
                    b |= mask << i * 4;
                }
                output_buffer.push(b).unwrap();
            }
            output_buffer.reverse();
            self.driver.write(&output_buffer)?;
            let mut input_buffer = Vec::<u8, MAX_CHARS>::new();
            input_buffer.resize(count, 0).unwrap();
            self.driver.read(&mut input_buffer)?;
            for i in 0..count {
                let hall = input_buffer[i] & 2 == 2;
                if Some(hall) != self.chars[i].prev_hall {
                    if let Some(prev_hall) = self.chars[i].prev_hall {
                        if prev_hall {
                            self.chars[i].homed = true;
                            info!("homed {} at {:?}", i, self.chars[i].position);
                            self.chars[i].position = Some(0);
                        }
                    }
                    self.chars[i].prev_hall = Some(hall);
                }
            }
            if done {
                break;
            }
        }
        self.driver.set_enabled(false);
        Ok(())
    }
}

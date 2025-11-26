use crate::driver::DriverModule;
use crate::error::Error;
use core::default::Default;
use core::ops::Index;
use core::{fmt, iter};
use embassy_time::Timer;
use heapless::{String, Vec};
use log::{error, info};
// use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;
// use unidecode::unidecode_char;

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
        info!("Counted {}", count);
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
                    calibration: *[1870, 1910, 1840, 1848, 1848, 1858, 1840, 1860, 1868, 1870]
                        .get(self.chars.len())
                        .unwrap_or(&0),
                })
                .ok()
                .unwrap();
        }
        for char in self.chars.iter_mut() {
            char.homed = false;
            char.position = None;
        }
        let mut flaps = Vec::<usize, MAX_CHARS>::new();

        // Attempt to assign each grapheme to one displayed character. This ensures diacritics
        // are handled together with the base code point.
        for grapheme_in in UnicodeSegmentation::graphemes(message, true) {
            // TODO: do this without allocation.
            // // Check for a matching grapheme with the same Unicode canonical normalization. This ensures
            // // graphemes with different code point sequences that should render identically are
            // // matched. For example, "\u00F1" (LATIN SMALL LETTER N WITH TILDE) and "\u006E\u0303"
            // // (LATIN SMALL LETTER N, COMBINING TILDE) should both use the same flap.
            // if let Some(matched) =
            //     common::letters_iter().position(|g| g.nfd().eq(grapheme_in.nfd()))
            // {
            //     flaps.push(matched).ok();
            //     continue;
            // }
            // // If we failed to find a canonical match, look for a compatible match. This will handle
            // // imperfect matches like "\u0190" (LATIN CAPITAL LETTER OPEN E ) for "\u2107" (EULER CONSTANT).
            // if let Some(matched) =
            //     common::letters_iter().position(|g| g.nfkd().eq(grapheme_in.nfkd()))
            // {
            //     flaps.push(matched).ok();
            //     continue;
            // }

            // A
            let success = false;
            for c in grapheme_in.chars() {
                if let Some(matched) = common::letters_iter()
                    .position(|g| g.len() == 1 && g.chars().next().unwrap() == c)
                {
                    flaps.push(matched).ok();
                }
                // for c in unidecode_char(c).chars() {
                //     if let Some(matched) = common::letters_iter()
                //         .position(|g| g.len() == 1 && g.chars().next().unwrap() == c)
                //     {
                //         flaps.push(matched).ok();
                //     }
                // }
            }
            if !success {
                flaps.push(0).ok();
            }
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
                        if false && *position == target {
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
            input_buffer.resize(count + 1, 0).unwrap();
            self.driver.read(&mut input_buffer)?;
            if input_buffer[count] != 0xFF {
                error!("Read error: bad terminator {}", input_buffer[count]);
            }
            for i in 0..count {
                let fault = input_buffer[i] & 1 == 1;
                let hall = input_buffer[i] & 2 == 2;
                let zeros = input_buffer[i] & !0b11;
                if zeros != 0 {
                    error!("Read error: bad bits {}", i);
                }
                if !fault {
                    error!("Motor fault {}", i);
                }
                if Some(hall) != self.chars[i].prev_hall {
                    if let Some(prev_hall) = self.chars[i].prev_hall {
                        if !prev_hall {
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

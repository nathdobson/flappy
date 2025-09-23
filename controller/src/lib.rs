#![no_std]
#![allow(dead_code, unused_imports)]
#![allow(unreachable_code)]
#![deny(unused_must_use)]
#![allow(unused_variables)]
#![feature(never_type)]

mod split_flap;
mod split_flap_display;
mod terminate;

use crate::split_flap::SplitFlap;
use crate::split_flap_display::SplitFlapDisplay;
use crate::terminate::TerminateResult;
use arduino_core::delay::{delay, delay_microseconds};
use arduino_core::pins::{
    AnalogInputPin, DigitalInputPin, DigitalOutputPin, NativeAnalogInputPin, NativeDigitalInputPin,
    NativeDigitalOutputPin,
};
use arduino_core::serial::Serial;
use arduino_core::sprintln;
use arduino_shift_output::{OutputRegister, SpiOutputRegister};
use arduino_stepper::{FOUR_PHASE_FULL, UnipolarStepper};
use arrayvec::{ArrayString, ArrayVec};
use common::LETTERS;
use core::iter::repeat_n;

const MODULE_COUNT: usize = 2;

#[arduino_core::entry]
fn main() {
    main_impl().ok();
}
fn main_impl() -> TerminateResult<()> {
    Serial::begin(112500);
    let data = NativeDigitalOutputPin::new(2);
    let latch = NativeDigitalOutputPin::new(3);
    let clock = NativeDigitalOutputPin::new(4);
    let hall_input = NativeDigitalInputPin::new(5);

    let register = SpiOutputRegister::<{ MODULE_COUNT * 8 }, _, _, _>::new(data, clock, latch);
    let mut steppers = ArrayVec::<_, MODULE_COUNT>::new();
    let mut hall_outputs = ArrayVec::<_, MODULE_COUNT>::new();
    for module in 0u16..MODULE_COUNT as u16 {
        hall_outputs.push(register.pin(module * 8 + 1));
        steppers.push(UnipolarStepper::new(
            [
                register.pin(module * 8 + 4),
                register.pin(module * 8 + 5),
                register.pin(module * 8 + 6),
                register.pin(module * 8 + 7),
            ],
            &FOUR_PHASE_FULL,
        ));
    }
    register.update();

    while Serial::available() == 0 {}
    Serial::read(&mut [0u8; 1]);
    sprintln!("Hello, world!");

    let mut display = SplitFlapDisplay::new(
        &register,
        steppers.into_inner().ok().unwrap(),
        hall_outputs.into_inner().ok().unwrap(),
        hall_input,
        LETTERS,
        2048,
        [1830, 1740],
        250,
        2000000,
        16,
        5,
    );
    display.run("NO")?;
    return Ok(());
    for char in LETTERS.chars() {
        sprintln!("Displaying {}", char);
        let mut str = ArrayString::<MODULE_COUNT>::new();
        for i in 0..MODULE_COUNT {
            str.push(char);
        }
        display.run(&str)?;
        if char == ' ' {
            delay(2000);
        } else {
            delay(300);
        }
    }
    Ok(())
    //
    // let message = "HI";
    // let targets = message
    //     .chars()
    //     .map(|x| {
    //         (LETTERS.chars().position(|y| x == y).unwrap() * steps_per_rotation
    //             / LETTERS.chars().count()
    //             + indexing)
    //             % steps_per_rotation
    //     })
    //     .collect::<ArrayVec<_, MODULE_COUNT>>()
    //     .into_inner()
    //     .unwrap();
    // let positions: [Option<usize>; MODULE_COUNT] = [None; MODULE_COUNT];
    // let previous_signal: [bool; MODULE_COUNT] = [true; MODULE_COUNT];
    // let mut current_sensor = 0;
    // for time in 0u64.. {
    //     let new_signal = signal.digital_read();
    //     current_sensor = (current_sensor + 1) % MODULE_COUNT;
    //     for sensor in 0..MODULE_COUNT {
    //         sensors[sensor].digital_write(current_sensor == sensor);
    //     }
    //     register.update();
    //     delay_microseconds(1);
    // }

    // let motor_to_use = 1;
    // sensors[motor_to_use].digital_write(true);
    // register.update();
    // while signal.digital_read() {
    //     if Serial::available() != 0 {
    //         return;
    //     }
    //     motors[motor_to_use].step(false);
    //     register.update();
    //     delay_microseconds(min_delay);
    // }
    // while !signal.digital_read() {
    //     if Serial::available() != 0 {
    //         return;
    //     }
    //     motors[motor_to_use].step(false);
    //     register.update();
    //     delay_microseconds(min_delay);
    // }
    // let mut prev = signal.digital_read();
    // for i in 0.. {
    //     if Serial::available() != 0 {
    //         return;
    //     }
    //     motors[motor_to_use].step(false);
    //     register.update();
    //     delay_microseconds(min_delay);
    //     if (i + indexing) % steps_per_letter == 0 {
    //         motors[motor_to_use].disable();
    //         register.update();
    //         delay(100);
    //         motors[motor_to_use].enable();
    //         register.update();
    //     }
    //     let next = signal.digital_read();
    //     if prev != next {
    //         sprintln!("{} -> {} at {}", prev, next, i % steps_per_rotation);
    //         prev = next;
    //     }
    // }

    // let mut readings = [false; MODULE_COUNT];
    // for i in 0.. {
    //     if Serial::available() != 0 {
    //         break;
    //     }
    //     let current = i % MODULE_COUNT;
    //     for module in 0..MODULE_COUNT {
    //         motors[module].step(false);
    //         sensors[module].digital_write(current == module);
    //     }
    //
    //     register.update();
    //     // delay_microseconds(500);
    //     let new = signal.digital_read();
    //     if readings[current] != new {
    //         sprintln!(
    //             "Change of {} from {} to {} at {}",
    //             current,
    //             readings[current],
    //             new,
    //             i % 4096
    //         );
    //         readings[current] = new;
    //     }
    //     delay_microseconds(1200);
    // }
}

#![no_std]
#![allow(dead_code, unused_imports)]
#![allow(unreachable_code)]
#![deny(unused_must_use)]
#![allow(unused_variables)]
#![feature(never_type)]
#![allow(unused_imports)]
extern crate alloc;

mod split_flap;
mod split_flap_display;
mod terminate;

use crate::split_flap::SplitFlap;
use crate::split_flap_display::SplitFlapDisplay;
use crate::terminate::{TerminateResult, check_terminate};
use alloc::format;
use arduino_core::delay::{delay, delay_microseconds};
use arduino_core::pins::{
    AnalogInputPin, DigitalInputPin, DigitalOutputPin, NativeAnalogInputPin, NativeDigitalInputPin,
    NativeDigitalOutputPin,
};
use arduino_core::serial::Serial;
use arduino_core::{sprint, sprintln};
use arduino_shift_output::{OutputRegister, SpiOutputRegister};
use arduino_stepper::{
    FOUR_PHASE_FULL, FOUR_PHASE_HALF, Stepper, StepperDirection, UnipolarStepper,
};
use arrayvec::{ArrayString, ArrayVec};
use common::LETTERS;
use core::iter::repeat_n;

#[arduino_core::entry]
fn main() {
    Serial::begin(112500);
    main_impl_uln2003a().ok();
    sprintln!("\n\n\nterminating...\n\n\n");
}

fn wait_for_start() {
    while Serial::available() == 0 {}
    Serial::read(&mut [0u8; 1]);
    sprintln!("Hello, world!");
}
fn main_impl_uln2003a() -> TerminateResult<()> {
    const MODULE_COUNT: usize = 4;

    let data = NativeDigitalOutputPin::new(2);
    let latch = NativeDigitalOutputPin::new(3);
    let clock = NativeDigitalOutputPin::new(4);
    let hall_input = NativeDigitalInputPin::new(5);
    let register = SpiOutputRegister::<{ MODULE_COUNT * 8 }, _, _, _>::new(data, clock, latch);
    register.update();
    wait_for_start();
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
    loop {
        check_terminate()?;
        for stepper in &mut steppers {
            stepper.step(StepperDirection::Reverse);
        }
        register.update();
        delay(10);
    }
    Ok(())
}

fn main_impl_drv8804() -> TerminateResult<()> {
    let data_in = NativeDigitalInputPin::new(0);
    let load = NativeDigitalOutputPin::new(1);
    let data_out = NativeDigitalOutputPin::new(2);
    let latch = NativeDigitalOutputPin::new(3);
    let clock = NativeDigitalOutputPin::new(4);
    let register = SpiOutputRegister::<4, _, _, _>::new(data_out, &clock, latch);
    register.update();
    wait_for_start();
    let mut stepper = UnipolarStepper::new(
        [
            register.pin(0),
            register.pin(1),
            register.pin(2),
            register.pin(3),
        ],
        &FOUR_PHASE_HALF,
    );
    loop {
        check_terminate()?;
        stepper.step(StepperDirection::Forward);
        register.update();

        load.digital_write(false);
        delay(1);
        load.digital_write(true);
        delay(1);

        for i in 0..9 {
            sprint!("{}", data_in.digital_read() as u8);
            clock.digital_write(true);
            clock.digital_write(false);
        }
        sprintln!();

        delay(1000);
    }

    // let data = NativeDigitalOutputPin::new(2);
    // let latch = NativeDigitalOutputPin::new(3);
    // let clock = NativeDigitalOutputPin::new(4);
    // let hall_input = NativeDigitalInputPin::new(5);
    //
    // let register = SpiOutputRegister::<{ MODULE_COUNT * 8 }, _, _, _>::new(data, clock, latch);
    // let mut steppers = ArrayVec::<_, MODULE_COUNT>::new();
    // let mut hall_outputs = ArrayVec::<_, MODULE_COUNT>::new();
    // for module in 0u16..MODULE_COUNT as u16 {
    //     hall_outputs.push(register.pin(module * 8 + 1));
    //     steppers.push(UnipolarStepper::new(
    //         [
    //             register.pin(module * 8 + 4),
    //             register.pin(module * 8 + 5),
    //             register.pin(module * 8 + 6),
    //             register.pin(module * 8 + 7),
    //         ],
    //         &FOUR_PHASE_FULL,
    //     ));
    // }
    // register.update();
    //
    // while Serial::available() == 0 {}
    // Serial::read(&mut [0u8; 1]);
    // sprintln!("Hello, world!");
    //
    // let mut display = SplitFlapDisplay::new(
    //     &register,
    //     steppers.into_inner().ok().unwrap(),
    //     hall_outputs.into_inner().ok().unwrap(),
    //     hall_input,
    //     LETTERS,
    //     2048,
    //     // [1830, 1740],
    //     [1760, 1845, 1750, 1740],
    //     250,
    //     4000000,
    //     4,
    //     0,
    // );
    // for x in LETTERS.chars().step_by(5) {
    //     display.run(&format!("{}{}{}{}", x, x, x, x))?;
    //     delay(1000);
    // }
    // for char in LETTERS.chars() {
    //     sprintln!("Displaying {}", char);
    //     let mut str = ArrayString::<MODULE_COUNT>::new();
    //     for i in 0..MODULE_COUNT {
    //         str.push(char);
    //     }
    //     display.run(&str)?;
    //     if char == ' ' {
    //         delay(2000);
    //     } else {
    //         delay(300);
    //     }
    // }
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

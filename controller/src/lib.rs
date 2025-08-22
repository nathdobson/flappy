#![no_std]
#![allow(dead_code, unused_imports)]
#![allow(unreachable_code)]
#![deny(unused_must_use)]

use arduino_core::delay::{delay, delay_microseconds};
use arduino_core::pins::{
    DigitalInputPin, DigitalOutputPin, NativeDigitalInputPin, NativeDigitalOutputPin,
};
use arduino_core::serial::Serial;
use arduino_core::sprintln;
use arduino_shift_output::ShiftOutputRegister;
use arduino_stepper::{FOUR_PHASE_HALF_STEP, Stepper};

#[arduino_core::entry]
fn main() {
    Serial::begin(112500);
    while Serial::available() == 0 {}
    Serial::read(&mut [0u8; 1]);
    sprintln!("Hello, world!");
    let data = NativeDigitalOutputPin::new(2);
    let latch = NativeDigitalOutputPin::new(3);
    let clock = NativeDigitalOutputPin::new(4);
    let signal = NativeDigitalInputPin::new(5);
    let register = ShiftOutputRegister::<8, _, _, _>::new(data, clock, latch);
    register.pin(0).digital_write(false);
    register.pin(1).digital_write(true);
    register.pin(2).digital_write(false);
    register.pin(3).digital_write(false);
    let mut motor = Stepper::new(
        [
            register.pin(4),
            register.pin(5),
            register.pin(6),
            register.pin(7),
        ],
        &FOUR_PHASE_HALF_STEP,
    );
    let mut old = signal.digital_read();
    for i in 0.. {
        if Serial::available() != 0 {
            break;
        }
        motor.step(false);
        register.update();
        let new = signal.digital_read();
        if old != new {
            sprintln!("Change from {} to {} at {}", old, new, i % 4096);
            old = new;
        }
        delay_microseconds(1200);
    }
}

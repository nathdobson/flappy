use crate::split_flap::SplitFlap;
use crate::terminate::{TerminateResult, check_terminate};
use arduino_core::delay::{delay, delay_microseconds, micros};
use arduino_core::pins::{DigitalInputPin, DigitalOutputPin};
use arduino_core::sprintln;
use arduino_shift::OutputRegister;
use arduino_stepper::Stepper;
use arrayvec::ArrayVec;

pub struct SplitFlapDisplay<'a, const N: usize, R, S, HO, HI> {
    register: &'a R,
    flaps: [SplitFlap<S, HO>; N],
    hall_input: HI,
    hall_ticks: u64,
    delay_micros: u32,
}

impl<'a, const N: usize, R: OutputRegister, S: Stepper, HO: DigitalOutputPin, HI: DigitalInputPin>
    SplitFlapDisplay<'a, N, R, S, HO, HI>
{
    pub fn new(
        register: &'a R,
        steppers: [S; N],
        halls: [HO; N],
        hall_input: HI,
        letters: &'static str,
        steps_per_rotation: usize,
        offsets: [usize; N],
        hall_ticks: u64,
        delay_micros: u32,
    ) -> Self {
        SplitFlapDisplay {
            register,
            flaps: steppers
                .into_iter()
                .zip(halls.into_iter())
                .zip(offsets)
                .enumerate()
                .map(|(index, ((stepper, hall), offset))| {
                    SplitFlap::new(index, stepper, hall, letters, steps_per_rotation, offset)
                })
                .collect::<ArrayVec<_, N>>()
                .into_inner()
                .ok()
                .unwrap(),
            hall_input,
            hall_ticks,
            delay_micros,
        }
    }
    pub fn run(&mut self, message: &str) -> TerminateResult<()> {
        let mut chars = [' '; N];
        for (i, c) in message.chars().enumerate().take(N) {
            chars[i] = c;
        }
        for (flap, c) in self.flaps.iter_mut().zip(chars.iter()) {
            flap.set_target(*c);
        }
        for hall_step in 0.. {
            let current_sensor = (hall_step % (N as u64)) as usize;
            for micro_step in 0..self.hall_ticks {
                check_terminate()?;
                let mut done = true;
                for (index, flap) in self.flaps.iter_mut().enumerate() {
                    done &= flap.step();
                    flap.set_hall_enabled(index == current_sensor);
                }
                self.register.update();
                if done {
                    return Ok(());
                }
                delay_microseconds(self.delay_micros);
            }
            let value = self.hall_input.digital_read();
            self.flaps[current_sensor].set_hall_value(value);
        }
        Ok(())
    }
}

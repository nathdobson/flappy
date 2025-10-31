use crate::split_flap::SplitFlap;
use crate::terminate::{TerminateResult, check_terminate};
use arduino_core::delay::{delay_microseconds, micros};
use arduino_core::pins::{DigitalInputPin, DigitalOutputPin};
use arduino_core::sprintln;
use arduino_shift_output::OutputRegister;
use arduino_stepper::Stepper;
use arrayvec::ArrayVec;

pub struct SplitFlapDisplay<'a, const N: usize, R, S, HO, HI> {
    register: &'a R,
    flaps: [SplitFlap<S, HO>; N],
    hall_input: HI,
    tick_micros: u32,
    hall_ticks: u64,
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
        tick_micros: u32,
        delay_nanos: u64,
        hall_ticks: u64,
        max_slips: usize,
    ) -> Self {
        SplitFlapDisplay {
            register,
            flaps: steppers
                .into_iter()
                .zip(halls.into_iter())
                .zip(offsets)
                .enumerate()
                .map(|(index, ((stepper, hall), offset))| {
                    SplitFlap::new(
                        index,
                        stepper,
                        hall,
                        letters,
                        steps_per_rotation,
                        offset,
                        delay_nanos,
                        max_slips,
                    )
                })
                .collect::<ArrayVec<_, N>>()
                .into_inner()
                .ok()
                .unwrap(),
            hall_input,
            tick_micros,
            hall_ticks,
        }
    }
    pub fn run(&mut self, message: &str) -> TerminateResult<()> {
        let mut chars = [' '; N];
        for (i, c) in message.chars().enumerate() {
            chars[i] = c;
        }
        for (flap, c) in self.flaps.iter_mut().zip(chars.iter()) {
            flap.set_target(*c);
        }
        let start_micros = micros();
        let mut prev_sensor_step = 0;
        let mut current_sensor = 0;
        for step in 0u64.. {
            check_terminate()?;
            let sensor_step = step / self.hall_ticks;
            if sensor_step != prev_sensor_step {
                prev_sensor_step = sensor_step;
                let value = self.hall_input.digital_read();
                self.flaps[current_sensor].set_hall_value(value);
                current_sensor = (sensor_step % (N as u64)) as usize;
            }
            let mut done = true;
            for (index, flap) in self.flaps.iter_mut().enumerate() {
                done &= flap.advance_nanos((self.tick_micros as u64) * 1000);
                flap.set_hall_enabled(index == current_sensor);
            }
            self.register.update();
            if done {
                break;
            }
            while micros().wrapping_sub(start_micros) < (step as u32) * (self.tick_micros as u32) {}
        }
        Ok(())
    }
}

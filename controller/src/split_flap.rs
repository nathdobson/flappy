use arduino_core::pins::DigitalOutputPin;
use arduino_core::sprintln;
use arduino_stepper::{Stepper, StepperDirection, UnipolarStepper};
use common::LETTERS;

pub struct SplitFlap<S, HO> {
    index: usize,
    stepper: S,
    hall_output: HO,
    letters: &'static str,
    steps_per_rotation: usize,
    offset: usize,
    delay_nanos: u64,
    target: Option<usize>,
    position: usize,
    homed: bool,
    step_countdown: u64,
    previous_hall: Option<bool>,
    slips: usize,
    max_slips: usize,
}

impl<S: Stepper, HO: DigitalOutputPin> SplitFlap<S, HO> {
    pub fn new(
        index: usize,
        stepper: S,
        hall_output: HO,
        letters: &'static str,
        steps_per_rotation: usize,
        offset: usize,
        delay_nanos: u64,
        max_slips: usize,
    ) -> Self {
        Self {
            index,
            stepper,
            hall_output,
            letters,
            steps_per_rotation,
            offset,
            delay_nanos,
            target: None,
            position: 0,
            homed: false,
            step_countdown: 0,
            previous_hall: None,
            slips: 0,
            max_slips,
        }
    }
    pub fn advance_nanos(&mut self, nanos: u64) -> bool {
        let Some(target) = self.target else {
            self.stepper.set_enabled(false);
            return true;
        };
        if self.homed && self.position == target {
            self.stepper.set_enabled(false);
            return true;
        }
        if let Some(new_countdown) = self.step_countdown.checked_sub(nanos) {
            self.step_countdown = new_countdown;
        } else {
            self.step_countdown = self.delay_nanos;
            self.stepper.step(StepperDirection::Reverse);
            self.position += 1;
        }
        false
    }
    pub fn set_target(&mut self, c: char) {
        self.step_countdown = self.delay_nanos;
        self.target = Some(
            (LETTERS.chars().position(|x| c == x).unwrap() * self.steps_per_rotation
                / LETTERS.chars().count()
                + self.offset)
                % self.steps_per_rotation,
        );
        self.slips += 1;
        if self.slips >= self.max_slips {
            self.position = 0;
            self.homed = false;
            self.slips = 0;
        }
    }
    pub fn set_hall_enabled(&mut self, enabled: bool) {
        self.hall_output.digital_write(enabled);
    }
    pub fn set_hall_value(&mut self, value: bool) {
        if self.previous_hall == Some(true) && !value {
            self.slips = 0;
            self.homed = true;
            self.position = 0;
            sprintln!("homed {}", self.index);
        }
        self.previous_hall = Some(value);
    }
}

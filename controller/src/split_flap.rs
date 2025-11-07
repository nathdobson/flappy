use arduino_core::pins::DigitalOutputPin;
use arduino_core::sprintln;
use arduino_stepper::{Stepper, StepperDirection, UnipolarStepper};
use common::LETTERS;

const FALL_STEPS: usize = 10;
const UNTWIST_STEPS: usize = 5;

enum Mode {
    Disabled,
    Flipping { target: usize },
    Falling { steps: usize },
    Untwisting { steps: usize },
}
pub struct SplitFlap<S, HO> {
    index: usize,
    stepper: S,
    hall_output: HO,
    letters: &'static str,
    steps_per_rotation: usize,
    offset: usize,
    position: isize,
    homed: bool,
    previous_hall: Option<bool>,
    step_countdown: usize,
    mode: Mode,
}

impl<S: Stepper, HO: DigitalOutputPin> SplitFlap<S, HO> {
    pub fn new(
        index: usize,
        stepper: S,
        hall_output: HO,
        letters: &'static str,
        steps_per_rotation: usize,
        offset: usize,
    ) -> Self {
        Self {
            index,
            stepper,
            hall_output,
            letters,
            steps_per_rotation,
            offset,
            position: 0,
            homed: false,
            previous_hall: None,
            step_countdown: 0,
            mode: Mode::Disabled,
        }
    }
    // pub fn remaining(&self) -> usize {
    //     match &self.mode {
    //         Mode::Flipping { target } => {
    //             if self.homed {
    //
    //             }else{
    //                 self.steps_per_rotation * 2
    //             }
    //         }
    //         _ => 0,
    //     }
    // }
    pub fn step(&mut self) -> bool {
        match &mut self.mode {
            Mode::Disabled => {
                self.stepper.set_enabled(false);
                return true;
            }
            Mode::Flipping { target } => {
                if self.homed && self.position == *target as isize {
                    self.mode = Mode::Falling { steps: FALL_STEPS };
                } else if let Some(new_countdown) = self.step_countdown.checked_sub(1) {
                    self.step_countdown = new_countdown;
                } else {
                    self.step_countdown = 0;
                    self.stepper.step(StepperDirection::Reverse);
                    self.position += 1;
                }
            }
            Mode::Falling { steps } => {
                if let Some(less) = steps.checked_sub(1) {
                    *steps = less;
                } else {
                    self.mode = Mode::Untwisting {
                        steps: UNTWIST_STEPS,
                    };
                }
            }
            Mode::Untwisting { steps } => {
                if let Some(less) = steps.checked_sub(1) {
                    *steps = less;
                    self.stepper.step(StepperDirection::Forward);
                    self.position -= 1;
                } else {
                    self.mode = Mode::Disabled;
                }
            }
        }
        false
    }
    pub fn set_target(&mut self, c: char) {
        let c = c.to_ascii_uppercase();
        self.step_countdown = 0;
        let target = (LETTERS.chars().position(|x| c == x).unwrap_or(0) * self.steps_per_rotation
            / LETTERS.chars().count()
            + self.offset)
            % self.steps_per_rotation;
        self.mode = Mode::Flipping { target };
    }
    pub fn set_hall_enabled(&mut self, enabled: bool) {
        self.hall_output.digital_write(enabled);
    }
    pub fn set_hall_value(&mut self, value: bool) {
        if self.previous_hall == Some(true) && !value {
            sprintln!(
                "homed {} at position {}",
                self.index,
                self.position as isize - 2048
            );
            self.homed = true;
            self.position = 0;
        }
        self.previous_hall = Some(value);
    }
}

#![no_std]

use core::error::Error;
use core::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct Report<E>(E);

impl<E> Report<E> {
    pub fn new(e: E) -> Self {
        Report(e)
    }
}

impl<E: Display + Error> Display for Report<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut error: &dyn Error = &self.0;
        loop {
            write!(f, "{}", error)?;
            if let Some(source) = error.source() {
                write!(f, ": ")?;
                error = source;
            } else {
                break;
            }
        }
        Ok(())
    }
}

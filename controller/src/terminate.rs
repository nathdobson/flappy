use arduino_core::serial::Serial;

pub struct Terminate;
pub type TerminateResult<T> = Result<T, Terminate>;

pub fn check_terminate() -> TerminateResult<()> {
    if Serial::available() == 0 {
        Ok(())
    } else {
        Err(Terminate)
    }
}

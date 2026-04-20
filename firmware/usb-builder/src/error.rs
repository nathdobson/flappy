use embassy_executor::SpawnError;
use thiserror::Error;

#[derive(Error,Debug)]
pub enum Error {
    #[error("Spawn error {0}")]
    SpawnError(#[from] SpawnError),
}
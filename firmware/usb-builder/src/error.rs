use embassy_executor::SpawnError;
use thiserror::Error;

#[derive(Error,Debug)]
pub enum Error {
    #[error("Spawn error")]
    SpawnError(#[from] SpawnError),
}
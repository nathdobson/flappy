use std::error::Error;
use std::fmt::Display;
use web_sys::UsbTransferStatus;
#[derive(Debug, Clone)]
pub struct TransferError(UsbTransferStatus);
impl Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self.0 {
            UsbTransferStatus::Ok => write!(f, "ok"),
            UsbTransferStatus::Stall => write!(f, "stall"),
            UsbTransferStatus::Babble => write!(f, "babble"),
            _ => write!(f, "{:?}", self.0),
        }
    }
}
impl From<UsbTransferStatus> for TransferError {
    fn from(status: UsbTransferStatus) -> Self {
        Self(status)
    }
}

impl Error for TransferError {}

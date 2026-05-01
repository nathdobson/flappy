use crate::protocol::ReasonCode;
use core::str::Utf8Error;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProtocolError {
    #[error("buffer full")]
    BufferFull,
    #[error("unexpected EOF")]
    UnexpectedEof,
    #[error("malformed")]
    Malformed,
    #[error("unsupported")]
    Unsupported,
    #[error("connect failed")]
    ConnectFailed(#[source] ReasonCode),
    #[error("exceeded send concurrency limit")]
    ExceededSendConcurrency,
    #[error("invalid utf8")]
    Utf8Error,
    #[error("exceeded receive concurrency limit")]
    ExceededRecvConcurrency,
    #[error("publish failed")]
    PublishFailed(#[source] ReasonCode),
    #[error("disconnected")]
    Disconnected(#[source] ReasonCode),
    #[error("deadline exceeded")]
    DeadlineExceeded,
}

impl From<Utf8Error> for ProtocolError {
    fn from(_: Utf8Error) -> Self {
        ProtocolError::Utf8Error
    }
}

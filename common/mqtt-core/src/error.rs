use crate::protocol::ReasonCode;
use core::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProtocolError {
    BufferFull,
    UnexpectedEof,
    Malformed,
    Unsupported,
    ConnectFailed(ReasonCode),
    ExceededSendConcurrency,
    BadUtf8,
    ExceededRecvConcurrency,
    PublishFailed(ReasonCode),
    Disconnected(ReasonCode),
    DeadlineExceeded,
}

impl Display for ProtocolError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            ProtocolError::BufferFull => write!(f, "buffer full"),
            ProtocolError::UnexpectedEof => write!(f, "unexpected EOF"),
            ProtocolError::Malformed => write!(f, "malformed"),
            ProtocolError::Unsupported => write!(f, "unsupported"),
            ProtocolError::ConnectFailed(r) => write!(f, "connect failed {}", r),
            ProtocolError::ExceededSendConcurrency => write!(f, "exceed send concurrency"),
            ProtocolError::BadUtf8 => write!(f, "bad UTF-8"),
            ProtocolError::ExceededRecvConcurrency => write!(f, "exceed recv concurrency"),
            ProtocolError::PublishFailed(r) => write!(f, "publish failed {}", r),
            ProtocolError::Disconnected(r) => write!(f, "disconnected {}", r),
            ProtocolError::DeadlineExceeded => write!(f, "deadline exceeded"),
        }
    }
}

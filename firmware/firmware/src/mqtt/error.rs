use embassy_net::dns;
use embassy_net::tcp::ConnectError;
use embedded_io::ErrorKind;
use mbedtls_rs::{SessionError, TlsError};
use protocol::error::{DnsError, EmbeddedIoErrorKind, MqttServiceError, TcpError, TlsAlertDescription, TlsAlertLevel};

pub fn convert_dns_error(error: dns::Error) -> MqttServiceError {
    MqttServiceError::DnsError(match error {
        dns::Error::InvalidName => DnsError::InvalidName,
        dns::Error::NameTooLong => DnsError::NameTooLong,
        dns::Error::Failed => DnsError::Failed,
    })
}

pub fn convert_tcp_error(error: ConnectError) -> MqttServiceError {
    MqttServiceError::TcpError(match error {
        ConnectError::InvalidState => TcpError::InvalidState,
        ConnectError::ConnectionReset => TcpError::ConnectionReset,
        ConnectError::TimedOut => TcpError::TimedOut,
        ConnectError::NoRoute => TcpError::NoRoute,
    })
}


fn convert_tls_io_error(e: ErrorKind) -> EmbeddedIoErrorKind {
    match e {
        ErrorKind::Other => EmbeddedIoErrorKind::Other,
        ErrorKind::NotFound => EmbeddedIoErrorKind::NotFound,
        ErrorKind::PermissionDenied => EmbeddedIoErrorKind::PermissionDenied,
        ErrorKind::ConnectionRefused => EmbeddedIoErrorKind::ConnectionRefused,
        ErrorKind::ConnectionReset => EmbeddedIoErrorKind::ConnectionReset,
        ErrorKind::ConnectionAborted => EmbeddedIoErrorKind::ConnectionAborted,
        ErrorKind::NotConnected => EmbeddedIoErrorKind::NotConnected,
        ErrorKind::AddrInUse => EmbeddedIoErrorKind::AddrInUse,
        ErrorKind::AddrNotAvailable => EmbeddedIoErrorKind::AddrNotAvailable,
        ErrorKind::BrokenPipe => EmbeddedIoErrorKind::BrokenPipe,
        ErrorKind::AlreadyExists => EmbeddedIoErrorKind::AlreadyExists,
        ErrorKind::InvalidInput => EmbeddedIoErrorKind::InvalidInput,
        ErrorKind::InvalidData => EmbeddedIoErrorKind::InvalidData,
        ErrorKind::TimedOut => EmbeddedIoErrorKind::TimedOut,
        ErrorKind::Interrupted => EmbeddedIoErrorKind::Interrupted,
        ErrorKind::Unsupported => EmbeddedIoErrorKind::Unsupported,
        ErrorKind::OutOfMemory => EmbeddedIoErrorKind::OutOfMemory,
        ErrorKind::WriteZero => EmbeddedIoErrorKind::WriteZero,
        _ => EmbeddedIoErrorKind::Unknown,
    }
}
pub fn convert_tls_error(error: TlsError) -> MqttServiceError {
    todo!();
}

pub fn convert_mqtt_error(error: mqtt_client::error::Error<SessionError>) -> MqttServiceError {
    todo!();
}
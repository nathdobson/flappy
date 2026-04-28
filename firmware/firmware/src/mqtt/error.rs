use embassy_net::dns;
use embassy_net::tcp::ConnectError;
use embedded_io::ErrorKind;
use embedded_tls::TlsError;
use embedded_tls::alert::{AlertDescription, AlertLevel};
use mqtt_client::error::Error;
use protocol::error::{
    DnsError, EmbeddedIoErrorKind, MqttServiceError, TcpError, TlsAlertDescription, TlsAlertLevel,
};

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

pub fn convert_alert_level(a: AlertLevel) -> TlsAlertLevel {
    match a {
        AlertLevel::Warning => TlsAlertLevel::Warning,
        AlertLevel::Fatal => TlsAlertLevel::Fatal,
    }
}

pub fn convert_alert_description(a: AlertDescription) -> TlsAlertDescription {
    match a {
        AlertDescription::CloseNotify => TlsAlertDescription::CloseNotify,
        AlertDescription::UnexpectedMessage => TlsAlertDescription::UnexpectedMessage,
        AlertDescription::BadRecordMac => TlsAlertDescription::BadRecordMac,
        AlertDescription::RecordOverflow => TlsAlertDescription::RecordOverflow,
        AlertDescription::HandshakeFailure => TlsAlertDescription::HandshakeFailure,
        AlertDescription::BadCertificate => TlsAlertDescription::BadCertificate,
        AlertDescription::UnsupportedCertificate => TlsAlertDescription::UnsupportedCertificate,
        AlertDescription::CertificateRevoked => TlsAlertDescription::CertificateRevoked,
        AlertDescription::CertificateExpired => TlsAlertDescription::CertificateExpired,
        AlertDescription::CertificateUnknown => TlsAlertDescription::CertificateUnknown,
        AlertDescription::IllegalParameter => TlsAlertDescription::IllegalParameter,
        AlertDescription::UnknownCa => TlsAlertDescription::UnknownCa,
        AlertDescription::AccessDenied => TlsAlertDescription::AccessDenied,
        AlertDescription::DecodeError => TlsAlertDescription::DecodeError,
        AlertDescription::DecryptError => TlsAlertDescription::DecryptError,
        AlertDescription::ProtocolVersion => TlsAlertDescription::ProtocolVersion,
        AlertDescription::InsufficientSecurity => TlsAlertDescription::InsufficientSecurity,
        AlertDescription::InternalError => TlsAlertDescription::InternalError,
        AlertDescription::InappropriateFallback => TlsAlertDescription::InappropriateFallback,
        AlertDescription::UserCanceled => TlsAlertDescription::UserCanceled,
        AlertDescription::MissingExtension => TlsAlertDescription::MissingExtension,
        AlertDescription::UnsupportedExtension => TlsAlertDescription::UnsupportedExtension,
        AlertDescription::UnrecognizedName => TlsAlertDescription::UnrecognizedName,
        AlertDescription::BadCertificateStatusResponse => {
            TlsAlertDescription::BadCertificateStatusResponse
        }
        AlertDescription::UnknownPskIdentity => TlsAlertDescription::UnknownPskIdentity,
        AlertDescription::CertificateRequired => TlsAlertDescription::CertificateRequired,
        AlertDescription::NoApplicationProtocol => TlsAlertDescription::NoApplicationProtocol,
    }
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
    MqttServiceError::TlsError(match error {
        TlsError::ConnectionClosed => protocol::error::TlsError::ConnectionClosed,
        TlsError::Unimplemented => protocol::error::TlsError::Unimplemented,
        TlsError::MissingHandshake => protocol::error::TlsError::MissingHandshake,
        TlsError::HandshakeAborted(level, description) => {
            protocol::error::TlsError::HandshakeAborted(
                convert_alert_level(level),
                convert_alert_description(description),
            )
        }
        TlsError::AbortHandshake(level, description) => protocol::error::TlsError::AbortHandshake(
            convert_alert_level(level),
            convert_alert_description(description),
        ),
        TlsError::IoError => protocol::error::TlsError::IoError,
        TlsError::InternalError => protocol::error::TlsError::InternalError,
        TlsError::InvalidRecord => protocol::error::TlsError::InvalidRecord,
        TlsError::UnknownContentType => protocol::error::TlsError::UnknownContentType,
        TlsError::InvalidNonceLength => protocol::error::TlsError::InvalidNonceLength,
        TlsError::InvalidTicketLength => protocol::error::TlsError::InvalidTicketLength,
        TlsError::UnknownExtensionType => protocol::error::TlsError::UnknownExtensionType,
        TlsError::InsufficientSpace => protocol::error::TlsError::InsufficientSpace,
        TlsError::InvalidHandshake => protocol::error::TlsError::InvalidHandshake,
        TlsError::InvalidCipherSuite => protocol::error::TlsError::InvalidCipherSuite,
        TlsError::InvalidSignatureScheme => protocol::error::TlsError::InvalidSignatureScheme,
        TlsError::InvalidSignature => protocol::error::TlsError::InvalidSignature,
        TlsError::InvalidExtensionsLength => protocol::error::TlsError::InvalidExtensionsLength,
        TlsError::InvalidSessionIdLength => protocol::error::TlsError::InvalidSessionIdLength,
        TlsError::InvalidSupportedVersions => protocol::error::TlsError::InvalidSupportedVersions,
        TlsError::InvalidApplicationData => protocol::error::TlsError::InvalidApplicationData,
        TlsError::InvalidKeyShare => protocol::error::TlsError::InvalidKeyShare,
        TlsError::InvalidCertificate => protocol::error::TlsError::InvalidCertificate,
        TlsError::InvalidCertificateEntry => protocol::error::TlsError::InvalidCertificateEntry,
        TlsError::InvalidCertificateRequest => protocol::error::TlsError::InvalidCertificateRequest,
        TlsError::UnableToInitializeCryptoEngine => {
            protocol::error::TlsError::UnableToInitializeCryptoEngine
        }
        TlsError::ParseError(error) => protocol::error::TlsError::ParseError,
        TlsError::OutOfMemory => protocol::error::TlsError::OutOfMemory,
        TlsError::CryptoError => protocol::error::TlsError::CryptoError,
        TlsError::EncodeError => protocol::error::TlsError::EncodeError,
        TlsError::DecodeError => protocol::error::TlsError::DecodeError,
        TlsError::Io(error) => protocol::error::TlsError::Io(convert_tls_io_error(error)),
        TlsError::InvalidPrivateKey => protocol::error::TlsError::InvalidPrivateKey,
    })
}

pub fn convert_mqtt_error(
    error: mqtt_client::error::Error<TlsError, TlsError>,
) -> MqttServiceError {
    match error {
        Error::WriteError(x) => convert_tls_error(x),
        Error::ReadError(x) => convert_tls_error(x),
        Error::ProtocolError(x) => MqttServiceError::MqttError(x),
    }
}

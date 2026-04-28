#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DnsError {
    InvalidName,
    NameTooLong,
    Failed,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TcpError {
    InvalidState,
    ConnectionReset,
    TimedOut,
    NoRoute,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TlsAlertLevel {
    Fatal,
    Warning,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TlsAlertDescription {
    CloseNotify,
    UnexpectedMessage,
    BadRecordMac,
    RecordOverflow,
    HandshakeFailure,
    BadCertificate,
    UnsupportedCertificate,
    CertificateRevoked,
    CertificateExpired,
    CertificateUnknown,
    IllegalParameter,
    UnknownCa,
    AccessDenied,
    DecodeError,
    DecryptError,
    ProtocolVersion,
    InsufficientSecurity,
    InternalError,
    InappropriateFallback,
    UserCanceled,
    MissingExtension,
    UnsupportedExtension,
    UnrecognizedName,
    BadCertificateStatusResponse,
    UnknownPskIdentity,
    CertificateRequired,
    NoApplicationProtocol,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TlsParseError {}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EmbeddedIoErrorKind {
    Other,
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    BrokenPipe,
    AlreadyExists,
    InvalidInput,
    InvalidData,
    TimedOut,
    Interrupted,
    Unsupported,
    OutOfMemory,
    WriteZero,
    Unknown,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TlsError {
    ConnectionClosed,
    Unimplemented,
    MissingHandshake,
    HandshakeAborted(TlsAlertLevel, TlsAlertDescription),
    AbortHandshake(TlsAlertLevel, TlsAlertDescription),
    IoError,
    InternalError,
    InvalidRecord,
    UnknownContentType,
    InvalidNonceLength,
    InvalidTicketLength,
    UnknownExtensionType,
    InsufficientSpace,
    InvalidHandshake,
    InvalidCipherSuite,
    InvalidSignatureScheme,
    InvalidSignature,
    InvalidExtensionsLength,
    InvalidSessionIdLength,
    InvalidSupportedVersions,
    InvalidApplicationData,
    InvalidKeyShare,
    InvalidCertificate,
    InvalidCertificateEntry,
    InvalidCertificateRequest,
    UnableToInitializeCryptoEngine,
    ParseError,
    OutOfMemory,
    CryptoError,
    EncodeError,
    DecodeError,
    Io(EmbeddedIoErrorKind),
    InvalidPrivateKey,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MqttServiceError {
    DnsError(DnsError),
    TcpError(TcpError),
    TlsError(TlsError),
    MqttError(mqtt_core::error::ProtocolError),
    Disconnected,
    DeadlineExceeded,
    NoCertificateListSha256,
    TopicTooLong,
    AllocError,
}

#[cfg(feature = "embassy-net")]
mod embassy_net_impls {
    use crate::error::{DnsError, MqttServiceError, TcpError};
    use embassy_net::{dns, tcp};

    impl From<dns::Error> for MqttServiceError {
        fn from(error: dns::Error) -> MqttServiceError {
            MqttServiceError::DnsError(match error {
                dns::Error::InvalidName => DnsError::InvalidName,
                dns::Error::NameTooLong => DnsError::NameTooLong,
                dns::Error::Failed => DnsError::Failed,
            })
        }
    }

    impl From<tcp::ConnectError> for MqttServiceError {
        fn from(error: tcp::ConnectError) -> MqttServiceError {
            MqttServiceError::TcpError(match error {
                tcp::ConnectError::InvalidState => TcpError::InvalidState,
                tcp::ConnectError::ConnectionReset => TcpError::ConnectionReset,
                tcp::ConnectError::TimedOut => TcpError::TimedOut,
                tcp::ConnectError::NoRoute => TcpError::NoRoute,
            })
        }
    }
}

#[cfg(feature = "embedded-tls")]
mod embedded_tls_impls {
    use crate::error::{MqttServiceError, TlsAlertDescription, TlsAlertLevel, TlsError};
    use embedded_tls::alert::{AlertDescription, AlertLevel};

    impl From<AlertLevel> for TlsAlertLevel {
        fn from(level: AlertLevel) -> TlsAlertLevel {
            match level {
                AlertLevel::Warning => TlsAlertLevel::Warning,
                AlertLevel::Fatal => TlsAlertLevel::Fatal,
            }
        }
    }
    impl From<AlertDescription> for TlsAlertDescription {
        fn from(description: AlertDescription) -> TlsAlertDescription {
            match description {
                AlertDescription::CloseNotify => TlsAlertDescription::CloseNotify,
                AlertDescription::UnexpectedMessage => TlsAlertDescription::UnexpectedMessage,
                AlertDescription::BadRecordMac => TlsAlertDescription::BadRecordMac,
                AlertDescription::RecordOverflow => TlsAlertDescription::RecordOverflow,
                AlertDescription::HandshakeFailure => TlsAlertDescription::HandshakeFailure,
                AlertDescription::BadCertificate => TlsAlertDescription::BadCertificate,
                AlertDescription::UnsupportedCertificate => {
                    TlsAlertDescription::UnsupportedCertificate
                }
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
                AlertDescription::InappropriateFallback => {
                    TlsAlertDescription::InappropriateFallback
                }
                AlertDescription::UserCanceled => TlsAlertDescription::UserCanceled,
                AlertDescription::MissingExtension => TlsAlertDescription::MissingExtension,
                AlertDescription::UnsupportedExtension => TlsAlertDescription::UnsupportedExtension,
                AlertDescription::UnrecognizedName => TlsAlertDescription::UnrecognizedName,
                AlertDescription::BadCertificateStatusResponse => {
                    TlsAlertDescription::BadCertificateStatusResponse
                }
                AlertDescription::UnknownPskIdentity => TlsAlertDescription::UnknownPskIdentity,
                AlertDescription::CertificateRequired => TlsAlertDescription::CertificateRequired,
                AlertDescription::NoApplicationProtocol => {
                    TlsAlertDescription::NoApplicationProtocol
                }
            }
        }
    }
    impl From<embedded_tls::TlsError> for MqttServiceError {
        fn from(error: embedded_tls::TlsError) -> MqttServiceError {
            MqttServiceError::TlsError(match error {
                embedded_tls::TlsError::ConnectionClosed => TlsError::ConnectionClosed,
                embedded_tls::TlsError::Unimplemented => TlsError::Unimplemented,
                embedded_tls::TlsError::MissingHandshake => TlsError::MissingHandshake,
                embedded_tls::TlsError::HandshakeAborted(level, description) => {
                    TlsError::HandshakeAborted(level.into(), description.into())
                }
                embedded_tls::TlsError::AbortHandshake(level, description) => {
                    TlsError::AbortHandshake(level.into(), description.into())
                }
                embedded_tls::TlsError::IoError => TlsError::IoError,
                embedded_tls::TlsError::InternalError => TlsError::InternalError,
                embedded_tls::TlsError::InvalidRecord => TlsError::InvalidRecord,
                embedded_tls::TlsError::UnknownContentType => TlsError::UnknownContentType,
                embedded_tls::TlsError::InvalidNonceLength => TlsError::InvalidNonceLength,
                embedded_tls::TlsError::InvalidTicketLength => TlsError::InvalidTicketLength,
                embedded_tls::TlsError::UnknownExtensionType => TlsError::UnknownExtensionType,
                embedded_tls::TlsError::InsufficientSpace => TlsError::InsufficientSpace,
                embedded_tls::TlsError::InvalidHandshake => TlsError::InvalidHandshake,
                embedded_tls::TlsError::InvalidCipherSuite => TlsError::InvalidCipherSuite,
                embedded_tls::TlsError::InvalidSignatureScheme => TlsError::InvalidSignatureScheme,
                embedded_tls::TlsError::InvalidSignature => TlsError::InvalidSignature,
                embedded_tls::TlsError::InvalidExtensionsLength => {
                    TlsError::InvalidExtensionsLength
                }
                embedded_tls::TlsError::InvalidSessionIdLength => TlsError::InvalidSessionIdLength,
                embedded_tls::TlsError::InvalidSupportedVersions => {
                    TlsError::InvalidSupportedVersions
                }
                embedded_tls::TlsError::InvalidApplicationData => TlsError::InvalidApplicationData,
                embedded_tls::TlsError::InvalidKeyShare => TlsError::InvalidKeyShare,
                embedded_tls::TlsError::InvalidCertificate => TlsError::InvalidCertificate,
                embedded_tls::TlsError::InvalidCertificateEntry => {
                    TlsError::InvalidCertificateEntry
                }
                embedded_tls::TlsError::InvalidCertificateRequest => {
                    TlsError::InvalidCertificateRequest
                }
                embedded_tls::TlsError::UnableToInitializeCryptoEngine => {
                    TlsError::UnableToInitializeCryptoEngine
                }
                embedded_tls::TlsError::ParseError(error) => TlsError::ParseError,
                embedded_tls::TlsError::OutOfMemory => TlsError::OutOfMemory,
                embedded_tls::TlsError::CryptoError => TlsError::CryptoError,
                embedded_tls::TlsError::EncodeError => TlsError::EncodeError,
                embedded_tls::TlsError::DecodeError => TlsError::DecodeError,
                embedded_tls::TlsError::Io(error) => TlsError::Io(error.into()),
                embedded_tls::TlsError::InvalidPrivateKey => TlsError::InvalidPrivateKey,
            })
        }
    }
}

#[cfg(feature = "embedded-io")]
mod embedded_io_impls {
    use crate::error::EmbeddedIoErrorKind;
    use embedded_io::ErrorKind;
    impl From<ErrorKind> for EmbeddedIoErrorKind {
        fn from(kind: ErrorKind) -> EmbeddedIoErrorKind {
            match kind {
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
    }
}

#[cfg(feature = "mqtt-client")]
mod mqtt_client_impls {
    use crate::error::MqttServiceError;

    impl<W, R> From<mqtt_client::Error<W, R>> for MqttServiceError
    where
        MqttServiceError: From<W> + From<R>,
    {
        fn from(error: mqtt_client::Error<W, R>) -> Self {
            match error {
                mqtt_client::Error::WriteError(x) => x.into(),
                mqtt_client::Error::ReadError(x) => x.into(),
                mqtt_client::Error::ProtocolError(x) => MqttServiceError::MqttError(x),
            }
        }
    }
}

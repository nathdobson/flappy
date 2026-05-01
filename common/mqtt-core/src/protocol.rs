use core::marker::PhantomData;
use strum::FromRepr;
use thiserror::Error;

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, FromRepr)]
#[repr(u8)]
pub enum PacketType {
    CONNECT = 1,
    CONNACK = 2,
    PUBLISH = 3,
    PUBACK = 4,
    PUBREC = 5,
    PUBREL = 6,
    PUBCOMP = 7,
    SUBSCRIBE = 8,
    SUBACK = 9,
    UNSUBSCRIBE = 10,
    UNSUBACK = 11,
    PINGREQ = 12,
    PINGRESP = 13,
    DISCONNECT = 14,
    AUTH = 15,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, FromRepr)]
#[repr(u8)]
pub enum Qos {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum PayloadFormatIndicator {
    Bytes = 0,
    Utf8 = 1,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, FromRepr)]
#[repr(u8)]
pub enum PropertyId {
    PayloadFormatIndicator = 0x01,
    MessageExpiryInterval = 0x02,
    ContentType = 0x03,
    ResponseTopic = 0x08,
    CorrelationData = 0x09,
    SessionExpiryInterval = 0x11,
    WillDelayInterval = 0x18,
    TopicAlias = 0x23,
    UserProperty = 0x26,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum WillProperty<'a> {
    WillDelayInterval(u32),
    PayloadFormatIndicator(PayloadFormatIndicator),
    MessageExpiryInterval(u32),
    ContentType(&'a str),
    ResponseTopic(&'a str),
    CorrelationData(&'a [u8]),
    UserProperty(&'a str, &'a str),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Will<'a> {
    pub qos: Qos,
    pub retain: bool,
    pub properties: &'a [Property<'a>],
    pub topic: &'a str,
    pub payload: &'a [u8],
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct ConnectPacket<'a> {
    pub proto_name: &'a str,
    pub proto_version: u8,
    pub clean_start: bool,
    pub will: Option<Will<'a>>,
    pub password: Option<&'a str>,
    pub username: Option<&'a str>,
    pub keep_alive: u16,
    pub client_id: &'a str,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, FromRepr, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum ReasonCode {
    #[error("success")]
    Success = 0,
    #[error("no matching subscribers")]
    NoMatchingSubscribers = 0x10,
    #[error("unspecified error")]
    UnspecifiedError = 0x80,
    #[error("malformed packet")]
    MalformedPacket = 0x81,
    #[error("protocol error")]
    ProtocolError = 0x82,
    #[error("implementation specific error")]
    ImplementationSpecificError = 0x83,
    #[error("unsupported protocol version")]
    UnsupportedProtocolVersion = 0x84,
    #[error("client identifier not valid")]
    ClientIdentifierNotValid = 0x85,
    #[error("bad username or password")]
    BadUsernameOrPassword = 0x86,
    #[error("not authorized")]
    NotAuthorized = 0x87,
    #[error("server unavailable")]
    ServerUnavailable = 0x88,
    #[error("server busy")]
    ServerBusy = 0x89,
    #[error("banned")]
    Banned = 0x8a,
    #[error("bad authenticated method")]
    BadAuthenticationMethod = 0x8c,
    #[error("topic name invalid")]
    TopicNameInvalid = 0x90,
    #[error("packet identifier in use")]
    PacketIdentifierInUse = 0x91,
    #[error("packet too large")]
    PacketTooLarge = 0x95,
    #[error("quote exceeded")]
    QuotaExceeded = 0x97,
    #[error("payload format invalid")]
    PayloadFormatInvalid = 0x99,
    #[error("retain not supported")]
    RetainNotSupported = 0x9a,
    #[error("QOS not supported")]
    QosNotSupported = 0x9b,
    #[error("use another server")]
    UseAnotherServer = 0x9c,
    #[error("server moved")]
    ServerMoved = 0x9d,
    #[error("connection rate exceeded")]
    ConnectionRateExceeded = 0x9f,
    #[error("disconnect with will message")]
    DisconnectWithWillMessage = 0x04,
    #[error("server shutting down")]
    ServerShuttingDown = 0x8b,
    #[error("keep alive timeout")]
    KeepAliveTimeout = 0x8d,
    #[error("session taken over")]
    SessionTakenOver = 0x8e,
    #[error("topic filter invalid")]
    TopicFilterInvalid = 0x8f,
    #[error("receive maximum exceeded")]
    ReceiveMaximumExceeded = 0x93,
    #[error("topic alias invalid")]
    TopicAliasInvalid = 0x94,
    #[error("message rate too high")]
    MessageRateTooHigh = 0x96,
    #[error("administrative action")]
    AdministrativeAction = 0x98,
    #[error("shared subscriptions not supported")]
    SharedSubscriptionsNotSupported = 0x9e,
    #[error("maximum connect time")]
    MaximumConnectTime = 0xa0,
    #[error("subscription identifiers not supported")]
    SubscriptionIdentifiersNotSupported = 0xa1,
    #[error("wildcard subscription not supported")]
    WildcardSubscriptionNotSupported = 0xa2,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum MaximumQos {
    AtMostOnce = 0,
    AtLeastOnce = 1,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum Property<'a> {
    SessionExpiryInterval(u32),
    ReceiveMaximum(u16),
    MaximumQos(MaximumQos),
    RetainAvailable(bool),
    MaximumPacketSize(u32),
    AssignedClientIdentifier(&'a str),
    TopicAliasMaximum(u16),
    ReasonString(&'a str),
    WildcardSubscriptionAvailable(bool),
    SubscriptionIdentifiersAvailable(bool),
    SharedSubscriptionAvailable(bool),
    ServerKeepAlive(u16),
    ResponseInformation(&'a str),
    ServerReference(&'a str),
    AuthenticationMethod(&'a str),
    AuthenticationData(&'a [u8]),
    PayloadFormatIndicator(PayloadFormatIndicator),
    MessageExpiryInterval(u32),
    TopicAlias(u16),
    ResponseTopic(&'a str),
    CorrelationData(&'a [u8]),
    UserProperty(&'a str, &'a str),
    SubscriptionIdentifier(u32),
    ContentType(&'a str),
    WillDelayInterval(u32),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct ConnackPacket<'a> {
    pub session_present: bool,
    pub reason_code: ReasonCode,
    pub properties: &'a [Property<'a>],
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct DisconnectPacket<'a> {
    pub reason: ReasonCode,
    pub phantom: PhantomData<&'a ()>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct PublishPacket<'a> {
    pub dup: bool,
    pub qos: Qos,
    pub retain: bool,
    pub topic: &'a str,
    pub packet_id: Option<u16>,
    pub properties: &'a [Property<'a>],
    pub payload: &'a [u8],
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[repr(u8)]
pub enum RetainHandling {
    Send = 0,
    SendForNew = 1,
    DoNotSend = 2,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct TopicFilter<'a> {
    pub topic_filter: &'a str,
    pub max_qos: Qos,
    pub non_local: bool,
    pub retain_handling: RetainHandling,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct SubscribePacket<'a> {
    pub packet_id: u16,
    pub properties: &'a [Property<'a>],
    pub topic_filters: &'a [TopicFilter<'a>],
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct SubackPacket<'a> {
    pub packet_id: u16,
    pub properties: &'a [Property<'a>],
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct PubackPacket<'a> {
    pub packet_id: u16,
    pub reason_code: ReasonCode,
    pub properties: &'a [Property<'a>],
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct PubrecPacket<'a> {
    pub packet_id: u16,
    pub reason_code: ReasonCode,
    pub properties: &'a [Property<'a>],
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct PingreqPacket {}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct PingrespPacket {}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum Packet<'a> {
    Connack(ConnackPacket<'a>),
    Connect(ConnectPacket<'a>),
    Disconnect(DisconnectPacket<'a>),
    Publish(PublishPacket<'a>),
    Subscribe(SubscribePacket<'a>),
    Suback(SubackPacket<'a>),
    Puback(PubackPacket<'a>),
    Pubrec(PubrecPacket<'a>),
    Pingreq(PingreqPacket),
    Pingresp(PingrespPacket),
}
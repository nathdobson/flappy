use core::fmt::{Display, Formatter};
use core::marker::PhantomData;
use strum::FromRepr;

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

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, FromRepr)]
#[repr(u8)]
pub enum ReasonCode {
    Success = 0,
    NoMatchingSubscribers = 0x10,
    UnspecifiedError = 0x80,
    MalformedPacket = 0x81,
    ProtocolError = 0x82,
    ImplementationSpecificError = 0x83,
    UnsupportedProtocolVersion = 0x84,
    ClientIdentifierNotValid = 0x85,
    BadUsernameOrPassword = 0x86,
    NotAuthorized = 0x87,
    ServerUnavailable = 0x88,
    ServerBusy = 0x89,
    Banned = 0x8a,
    BadAuthenticationMethod = 0x8c,
    TopicNameInvalid = 0x90,
    PacketIdentifierInUse = 0x91,
    PacketTooLarge = 0x95,
    QuotaExceeded = 0x97,
    PayloadFormatInvalid = 0x99,
    RetainNotSupported = 0x9a,
    QosNotSupported = 0x9b,
    UseAnotherServer = 0x9c,
    ServerMoved = 0x9d,
    ConnectionRateExceeded = 0x9f,
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

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, FromRepr)]
#[repr(u8)]
pub enum DisconnectReason {
    NormalDisconnection = 0x00,
    DisconnectWithWillMessage = 0x04,
    UnspecifiedError = 0x80,
    MalformedPacket = 0x81,
    ProtocolError = 0x82,
    ImplementationSpecificError = 0x83,
    NotAuthorized = 0x87,
    ServerBusy = 0x89,
    ServerShuttingDown = 0x8b,
    KeepAliveTimeout = 0x8d,
    SessionTakenOver = 0x8e,
    TopicFilterInvalid = 0x8f,
    TopicNameInvalid = 0x90,
    ReceiveMaximumExceeded = 0x93,
    TopicAliasInvalid = 0x94,
    PacketTooLarge = 0x95,
    MessageRateTooHigh = 0x96,
    QuotaExceeded = 0x97,
    AdministrativeAction = 0x98,
    PayloadFormatInvalid = 0x99,
    RetainNotSupported = 0x9a,
    QosNotSupported = 0x9b,
    UseAnotherServer = 0x9c,
    ServerMoved = 0x9d,
    SharedSubscriptionsNotSupported = 0x9e,
    ConnectionRateExceeded = 0x9f,
    MaximumConnectTime = 0xa0,
    SubscriptionIdentifiersNotSupported = 0xa1,
    WildcardSubscriptionNotSupported = 0xa2,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct DisconnectPacket<'a> {
    pub reason: DisconnectReason,
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

impl Display for ReasonCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

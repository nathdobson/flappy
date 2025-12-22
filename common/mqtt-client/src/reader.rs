use crate::error::{Error, ProtocolError};
use crate::protocol::{
    ConnackPacket, ConnectPacket, DisconnectPacket, Packet, PacketType, PingrespPacket, Property,
    PropertyId, PubackPacket, PublishPacket, PubrecPacket, Qos, ReasonCode, SubackPacket,
};
use arena::Arena;
use core::marker::PhantomData;
use embedded_io_async::{BufRead, ErrorType, Read, ReadExactError};
use heapless::VecView;

pub struct MqttReader<R> {
    inner: R,
}

impl<R: Read> MqttReader<R> {
    pub fn new(inner: R) -> Self {
        MqttReader { inner }
    }
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error<R::Error>> {
        Ok(self.inner.read(buf).await.map_err(Error::NetworkError)?)
    }
    async fn read_u8(&mut self) -> Result<u8, Error<R::Error>> {
        let mut buf = [0; 1];
        if self.read(&mut buf).await? == 0 {
            return Err(ProtocolError::UnexpectedEof.into());
        };
        Ok(buf[0])
    }
    async fn read_varint(&mut self) -> Result<u8, Error<R::Error>> {
        let mut value = 0;
        for index in 0..4 {
            let b = self.read_u8().await?;
            value |= (b & 127) << (index * 7);
            if b & 128 == 0 {
                break;
            }
        }
        Ok(value)
    }
    pub async fn read_packet<'ar>(
        &mut self,
        arena: &'ar Arena,
    ) -> Result<Packet<'ar>, Error<R::Error>> {
        let b = self.read_u8().await?;
        let flags = b & 0x0F;
        let kind = PacketType::from_repr(b >> 4).ok_or(ProtocolError::Malformed)?;
        let length = self.read_varint().await?;
        let mut packet: &'ar mut [u8] = arena.alloc_bytes(length as usize)?;
        self.inner
            .read_exact(&mut packet)
            .await
            .map_err(|e| match e {
                ReadExactError::UnexpectedEof => ProtocolError::UnexpectedEof.into(),
                ReadExactError::Other(e) => Error::NetworkError(e),
            })?;
        let parser: PacketParser = PacketParser::new(packet, arena, kind, flags);
        let packet: Packet<'ar> = parser.parse()?;
        Ok(packet)
    }
}

struct PacketParser<'ar> {
    kind: PacketType,
    flags: u8,
    buffer: &'ar [u8],
    arena: &'ar Arena,
}

impl<'ar> PacketParser<'ar> {
    pub fn new(buffer: &'ar [u8], arena: &'ar Arena, kind: PacketType, flags: u8) -> Self {
        PacketParser {
            kind,
            flags,
            buffer,
            arena,
        }
    }
    pub fn read(&mut self, count: usize) -> Result<&'ar [u8], ProtocolError> {
        let (ret, rem) = self
            .buffer
            .split_at_checked(count)
            .ok_or(ProtocolError::UnexpectedEof)?;
        self.buffer = rem;
        Ok(ret)
    }
    pub fn read_fixed<const N: usize>(&mut self) -> Result<&'ar [u8; N], ProtocolError> {
        Ok(self
            .read(N)?
            .try_into()
            .ok()
            .ok_or(ProtocolError::UnexpectedEof)?)
    }
    pub fn read_u8(&mut self) -> Result<u8, ProtocolError> {
        Ok(self.read_fixed::<1>()?[0])
    }
    pub fn read_u16(&mut self) -> Result<u16, ProtocolError> {
        Ok(u16::from_be_bytes(*self.read_fixed::<2>()?))
    }
    pub fn read_bytes(&mut self) -> Result<&'ar [u8], ProtocolError> {
        let len = self.read_u16()? as usize;
        self.read(len)
    }
    pub fn read_string(&mut self) -> Result<&'ar str, ProtocolError> {
        Ok(str::from_utf8(self.read_bytes()?).map_err(|e| ProtocolError::BadUtf8)?)
    }
    pub fn parse_connack(&mut self) -> Result<ConnackPacket<'ar>, ProtocolError> {
        let session_present = self.read_u8()? & 1 == 1;
        let reason_code = ReasonCode::from_repr(self.read_u8()?).ok_or(ProtocolError::Malformed)?;
        let properties = self.read_u8()?;
        Ok(ConnackPacket {
            session_present,
            reason_code,
            properties: &[],
        })
    }
    pub fn parse_disconnect(&mut self) -> Result<DisconnectPacket<'ar>, ProtocolError> {
        let reason = ReasonCode::from_repr(self.read_u8()?).ok_or(ProtocolError::Malformed)?;
        Ok(DisconnectPacket {
            reason,
            phantom: PhantomData,
        })
    }
    pub fn parse_suback(&mut self) -> Result<SubackPacket<'ar>, ProtocolError> {
        let packet_id = self.read_u16()?;
        let properties: &'ar [Property] = self.parse_properties()?;
        Ok(SubackPacket {
            packet_id,
            properties,
        })
    }
    pub fn parse_publish(&mut self) -> Result<PublishPacket<'ar>, ProtocolError> {
        let retain = self.flags & 1 == 1;
        let qos = Qos::from_repr((self.flags >> 1) & 0b11).ok_or(ProtocolError::Malformed)?;
        let dup = (self.flags >> 3) == 1;
        let topic = self.read_string()?;
        let packet_id = if qos != Qos::AtMostOnce {
            Some(self.read_u16()?)
        } else {
            None
        };
        let properties = self.parse_properties()?;
        let payload = self.buffer;
        Ok(PublishPacket {
            dup,
            qos,
            retain,
            topic,
            packet_id: None,
            properties,
            payload,
        })
    }
    pub fn parse_puback(&mut self) -> Result<PubackPacket<'ar>, ProtocolError> {
        let packet_id = self.read_u16()?;
        if self.buffer.is_empty() {
            return Ok(PubackPacket {
                packet_id,
                reason_code: ReasonCode::Success,
                properties: &[],
            });
        }
        let reason_code = ReasonCode::from_repr(self.read_u8()?).ok_or(ProtocolError::Malformed)?;
        let properties: &'ar [Property] = self.parse_properties()?;
        Ok(PubackPacket {
            packet_id,
            reason_code,
            properties,
        })
    }
    pub fn parse_pubrec(&mut self) -> Result<PubrecPacket<'ar>, ProtocolError> {
        let packet_id = self.read_u16()?;
        if self.buffer.is_empty() {
            return Ok(PubrecPacket {
                packet_id,
                reason_code: ReasonCode::Success,
                properties: &[],
            });
        }
        let reason_code = ReasonCode::from_repr(self.read_u8()?).ok_or(ProtocolError::Malformed)?;
        let properties: &'ar [Property] = self.parse_properties()?;
        Ok(PubrecPacket {
            packet_id,
            reason_code,
            properties,
        })
    }
    pub fn parse_pingresp(&mut self) -> Result<PingrespPacket, ProtocolError> {
        Ok(PingrespPacket {})
    }
    pub fn parse_properties(&mut self) -> Result<&'ar [Property<'ar>], ProtocolError> {
        let properties = self.read_u8()?;
        for p in 0..properties {
            let id = self.read_u8()?;
            match PropertyId::from_repr(id).ok_or(ProtocolError::Malformed)? {
                PropertyId::PayloadFormatIndicator => todo!(),
                PropertyId::MessageExpiryInterval => todo!(),
                PropertyId::ContentType => todo!(),
                PropertyId::ResponseTopic => todo!(),
                PropertyId::CorrelationData => todo!(),
                PropertyId::SessionExpiryInterval => todo!(),
                PropertyId::WillDelayInterval => todo!(),
                PropertyId::TopicAlias => todo!(),
                PropertyId::UserProperty => todo!(),
            }
        }
        Ok(&[])
    }
    pub fn parse(mut self) -> Result<Packet<'ar>, ProtocolError> {
        match self.kind {
            PacketType::CONNACK => Ok(Packet::Connack(self.parse_connack()?)),
            PacketType::DISCONNECT => Ok(Packet::Disconnect(self.parse_disconnect()?)),
            PacketType::SUBACK => Ok(Packet::Suback(self.parse_suback()?)),
            PacketType::PUBLISH => Ok(Packet::Publish(self.parse_publish()?)),
            PacketType::PUBACK => Ok(Packet::Puback(self.parse_puback()?)),
            PacketType::PUBREC => Ok(Packet::Pubrec(self.parse_pubrec()?)),
            PacketType::PINGRESP => Ok(Packet::Pingresp(self.parse_pingresp()?)),
            _ => todo!("{:?}", self.kind),
        }
    }
}

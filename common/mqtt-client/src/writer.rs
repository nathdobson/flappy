use crate::error::Error;
use crate::varint::encode_varint;
use core::marker::PhantomData;
use embedded_io_async::Write;
use heapless::deque::DequeView;
use heapless::{Deque, Vec, VecView};
use mqtt_core::error::ProtocolError;
use mqtt_core::protocol::{
    ConnectPacket, Packet, PacketType, PingreqPacket, Property, PropertyId, PubackPacket,
    PublishPacket, SubscribePacket, WillProperty,
};

const FIXED_HEADER_RESERVATION: usize = 5;

pub struct MqttWriter<W, const SEND_CAP: usize> {
    inner: W,
    packet: Vec<u8, SEND_CAP>,
}

pub struct MqttPacketBuilder<'a> {
    typ: PacketType,
    flags: u8,
    packet_start: usize,
    packet: &'a mut VecView<u8>,
}

impl<const SEND_CAP: usize, W: Write> MqttWriter<W, SEND_CAP> {
    pub fn new(inner: W) -> Self {
        MqttWriter {
            inner,
            packet: Vec::new(),
        }
    }
    fn compute_flags(packet: &Packet<'_>) -> u8 {
        match packet {
            Packet::Connack(_) => 0,
            Packet::Connect(_) => 0,
            Packet::Disconnect(_) => 0,
            Packet::Publish(p) => {
                let mut flags = 0;
                if p.retain {
                    flags |= 1;
                }
                flags |= (p.qos as u8) << 1;
                if p.dup {
                    flags |= 8;
                }
                flags
            }
            Packet::Subscribe(p) => 2,
            Packet::Suback(_) => 0,
            Packet::Puback(_) => 0,
            Packet::Pubrec(_) => 0,
            Packet::Pingreq(_) => 0,
            Packet::Pingresp(_) => 0,
        }
    }
    pub async fn send_packet(&mut self, packet: &Packet<'_>) -> Result<(), Error<W::Error, !>> {
        self.packet.clear();
        self.packet.extend_from_slice(&[0u8; FIXED_HEADER_RESERVATION])?;
        let mut builder = MqttPacketBuilder {
            typ: match packet {
                Packet::Connack(_) => PacketType::CONNACK,
                Packet::Connect(_) => PacketType::CONNECT,
                Packet::Disconnect(_) => PacketType::DISCONNECT,
                Packet::Publish(_) => PacketType::PUBLISH,
                Packet::Subscribe(_) => PacketType::SUBSCRIBE,
                Packet::Suback(_) => PacketType::SUBACK,
                Packet::Puback(_) => PacketType::PUBACK,
                Packet::Pubrec(_) => PacketType::PUBREC,
                Packet::Pingreq(_) => PacketType::PINGREQ,
                Packet::Pingresp(_) => PacketType::PINGRESP,
            },
            flags: Self::compute_flags(&packet),
            packet_start: FIXED_HEADER_RESERVATION,
            packet: self.packet.as_mut_view(),
        };
        builder.write_packet(packet)?;
        let buf = builder.finish()?;
        self.inner
            .write_all(buf)
            .await
            .map_err(Error::WriteError)?;
        self.inner.flush().await.map_err(Error::WriteError)?;
        Ok(())
    }
}

impl<'a> MqttPacketBuilder<'a> {
    pub fn write_prefix(&mut self, data: &[u8]) -> Result<(), ProtocolError> {
        let new_start = self
            .packet_start
            .checked_sub(data.len())
            .ok_or(ProtocolError::BufferFull)?;
        self.packet[new_start..self.packet_start].copy_from_slice(data);
        self.packet_start = new_start;
        Ok(())
    }
    pub fn finish(mut self) -> Result<&'a [u8], ProtocolError> {
        let length = encode_varint(
            (self.packet.len() - self.packet_start)
                .try_into()
                .ok()
                .ok_or(ProtocolError::BufferFull)?,
        );
        self.write_prefix(&length)?;
        self.write_prefix(&[((self.typ as u8) << 4) | self.flags & 0b1111])?;
        Ok(&self.packet[self.packet_start..])
    }
    pub fn write(&mut self, data: &[u8]) -> Result<(), ProtocolError> {
        Ok(self
            .packet
            .extend_from_slice(data)
            .map_err(|_| ProtocolError::BufferFull)?)
    }
    pub fn write_u8(&mut self, data: u8) -> Result<(), ProtocolError> {
        self.write(&[data])
    }
    pub fn write_u16(&mut self, data: u16) -> Result<(), ProtocolError> {
        self.write(&data.to_be_bytes())
    }
    pub fn write_u32(&mut self, data: u32) -> Result<(), ProtocolError> {
        self.write(&data.to_be_bytes())
    }
    pub fn write_varint(&mut self, data: u32) -> Result<(), ProtocolError> {
        Ok(self.write(&encode_varint(data))?)
    }
    pub fn write_bytes(&mut self, data: &[u8]) -> Result<(), ProtocolError> {
        let len: u16 = data
            .len()
            .try_into()
            .ok()
            .ok_or(ProtocolError::BufferFull)?;
        self.write(&len.to_be_bytes())?;
        self.write(data)?;
        Ok(())
    }
    pub fn write_string(&mut self, s: &str) -> Result<(), ProtocolError> {
        self.write_bytes(s.as_bytes())?;
        Ok(())
    }
    pub fn write_properties(&mut self, properties: &[Property]) -> Result<(), ProtocolError> {
        self.write_varint(
            properties
                .len()
                .try_into()
                .ok()
                .ok_or(ProtocolError::BufferFull)?,
        )?;
        for prop in properties {
            self.write_property(prop)?;
        }
        Ok(())
    }
    pub fn write_property(&mut self, prop: &Property) -> Result<(), ProtocolError> {
        match prop {
            Property::PayloadFormatIndicator(x) => {
                self.write_u8(PropertyId::PayloadFormatIndicator as u8)?;
                self.write_u8(*x as u8)?;
            }
            Property::MessageExpiryInterval(x) => {
                self.write_u8(PropertyId::MessageExpiryInterval as u8)?;
                self.write_u32(*x)?;
            }
            Property::TopicAlias(x) => {
                self.write_u8(PropertyId::TopicAlias as u8)?;
                self.write_u16(*x)?;
            }
            Property::ResponseTopic(x) => {
                self.write_u8(PropertyId::ResponseTopic as u8)?;
                self.write_string(x)?;
            }
            Property::CorrelationData(x) => {
                self.write_u8(PropertyId::CorrelationData as u8)?;
                self.write_bytes(x)?;
            }
            Property::WillDelayInterval(x) => {
                self.write_u8(PropertyId::WillDelayInterval as u8)?;
                self.write_u32(*x)?;
            }
            Property::ContentType(x) => {
                self.write_u8(PropertyId::ContentType as u8)?;
                self.write_string(x)?;
            }
            Property::UserProperty(k, v) => {
                self.write_u8(PropertyId::UserProperty as u8)?;
                self.write_string(k)?;
                self.write_string(v)?;
            }
            Property::SessionExpiryInterval(x) => {
                self.write_u8(PropertyId::SessionExpiryInterval as u8)?;
                self.write_u32(*x)?;
            }
            _ => todo!(),
        }
        Ok(())
    }
    pub fn write_connect(&mut self, connect: &ConnectPacket<'_>) -> Result<(), ProtocolError> {
        self.write_string(connect.proto_name)?;
        self.write(&[connect.proto_version])?;
        let mut flags = 0;
        if connect.clean_start {
            flags |= 2;
        }
        if let Some(will) = &connect.will {
            if connect.will.is_some() {
                flags |= 4;
            }
            flags |= (will.qos as u8) << 3;
            if will.retain {
                flags |= 32;
            }
        }
        if connect.password.is_some() {
            flags |= 64;
        }
        if connect.username.is_some() {
            flags |= 128;
        }
        self.write(&[flags])?;
        self.write_u16(connect.keep_alive)?;
        self.write_varint(0)?;
        self.write_string(connect.client_id)?;
        if let Some(will) = &connect.will {
            self.write_properties(will.properties)?;
            self.write_string(will.topic)?;
            self.write_bytes(will.payload)?;
        }
        if let Some(username) = connect.username {
            self.write_string(username)?;
        }
        if let Some(password) = connect.password {
            self.write_string(password)?;
        }
        Ok(())
    }
    pub fn write_publish(&mut self, publish: &PublishPacket<'_>) -> Result<(), ProtocolError> {
        self.write_string(publish.topic)?;
        if let Some(packet_id) = publish.packet_id {
            self.write_u16(packet_id)?;
        }
        self.write_properties(&publish.properties)?;
        self.write(publish.payload)?;
        Ok(())
    }
    pub fn write_subscribe(
        &mut self,
        subscribe: &SubscribePacket<'_>,
    ) -> Result<(), ProtocolError> {
        self.write_u16(subscribe.packet_id)?;
        self.write_properties(subscribe.properties)?;
        for topic_filter in subscribe.topic_filters {
            self.write_string(topic_filter.topic_filter)?;
            let mut options = 0;
            options |= topic_filter.max_qos as u8;
            if topic_filter.non_local {
                options |= 4;
            }
            options |= (topic_filter.retain_handling as u8) << 3;
            self.write_u8(options)?;
        }
        Ok(())
    }
    pub fn write_puback(&mut self, puback: &PubackPacket<'_>) -> Result<(), ProtocolError> {
        todo!();
    }
    pub fn write_pingreq(&mut self, pingreq: &PingreqPacket) -> Result<(), ProtocolError> {
        Ok(())
    }
    pub fn write_packet(&mut self, packet: &Packet<'_>) -> Result<(), ProtocolError> {
        match packet {
            Packet::Connack(p) => Err(ProtocolError::Unsupported),
            Packet::Connect(p) => self.write_connect(p),
            Packet::Disconnect(p) => Err(ProtocolError::Unsupported),
            Packet::Publish(p) => self.write_publish(p),
            Packet::Subscribe(p) => self.write_subscribe(p),
            Packet::Suback(_) => Err(ProtocolError::Unsupported),
            Packet::Puback(p) => self.write_puback(p),
            Packet::Pubrec(p) => Err(ProtocolError::Unsupported),
            Packet::Pingreq(p) => self.write_pingreq(p),
            Packet::Pingresp(_) => Err(ProtocolError::Unsupported),
        }
    }
}

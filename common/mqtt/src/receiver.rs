use crate::error::{Error, ProtocolError};
use crate::proto::Packet;
use crate::reader::MqttReader;
use crate::sender::AckToken;
use arena::Arena;
use embedded_io_async::Read;

pub struct MqttReceiver<R> {
    reader: MqttReader<R>,
}

impl<R: Read> MqttReceiver<R> {
    pub fn new(read: R) -> MqttReceiver<R> {
        MqttReceiver {
            reader: MqttReader::new(read),
        }
    }
    pub async fn receive<'ar>(
        &mut self,
        arena: &'ar Arena,
    ) -> Result<(AckToken, Packet<'ar>), Error<R::Error>> {
        let packet = self.reader.read_packet(arena).await?;
        let token = match &packet {
            Packet::Connack(connack) => AckToken::Connack(connack.reason_code),
            Packet::Connect(_) => return Err(ProtocolError::Malformed.into()),
            Packet::Disconnect(_) => AckToken::Disconnect,
            Packet::Publish(p) => AckToken::Publish(p.packet_id),
            Packet::Subscribe(s) => return Err(ProtocolError::Malformed.into()),
            Packet::Suback(s) => AckToken::Suback(s.packet_id),
            Packet::Puback(s) => AckToken::Puback(s.packet_id, s.reason_code),
            Packet::Pubrec(s) => AckToken::Pubrec(s.packet_id, s.reason_code),
            Packet::Pingreq(s) => return Err(ProtocolError::Malformed.into()),
            Packet::Pingresp(s) => AckToken::Pingresp,
        };
        Ok((token, packet))
    }
}

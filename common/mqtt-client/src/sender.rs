use crate::error::Error;
use crate::writer::MqttWriter;
use core::cell::{Cell, RefCell};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embedded_io_async::Write;
use heapless::{Vec, VecView};
use mqtt_core::error::ProtocolError;
use mqtt_core::protocol::{
    ConnectPacket, Packet, PingreqPacket, PublishPacket, Qos, ReasonCode, RetainHandling,
    SubscribePacket, TopicFilter,
};

pub struct MqttSender<W, const SEND_CAP: usize, const RECV_CONC: usize, const SEND_CONC: usize> {
    writer: Mutex<NoopRawMutex, MqttWriter<W, SEND_CAP>>,
    connect_started: Cell<bool>,
    connack: Signal<NoopRawMutex, ReasonCode>,
    ping_mutex: Mutex<NoopRawMutex, ()>,
    pingresp: Signal<NoopRawMutex, ()>,
    send_acks: Channel<NoopRawMutex, SendAckToken, RECV_CONC>,
    packet_id_signals: [Signal<NoopRawMutex, RecvAckToken>; SEND_CONC],
    free_packet_ids: RefCell<Vec<u16, SEND_CONC>>,
    disconnect: Signal<NoopRawMutex, ReasonCode>,
}

pub enum AckToken {
    Connack(ReasonCode),
    Disconnect(ReasonCode),
    Suback(u16),
    Publish(Option<u16>),
    Puback(u16, ReasonCode),
    Pubrec(u16, ReasonCode),
    Pingresp,
}

enum SendAckToken {
    Publish(u16),
}

enum RecvAckToken {
    Suback,
    Puback(ReasonCode),
    Pubrec(ReasonCode),
}

impl<W: Write, const SEND_CAP: usize, const RECV_CONC: usize, const SEND_CONC: usize>
    MqttSender<W, SEND_CAP, RECV_CONC, SEND_CONC>
{
    pub fn new(write: W) -> Self {
        MqttSender {
            writer: Mutex::new(MqttWriter::new(write)),
            connect_started: Cell::new(false),
            connack: Signal::new(),
            ping_mutex: Mutex::new(()),
            pingresp: Signal::new(),
            send_acks: Channel::new(),
            packet_id_signals: [const { Signal::new() }; SEND_CONC],
            free_packet_ids: RefCell::new((1u16..SEND_CONC as u16 + 1).collect()),
            disconnect: Signal::new(),
        }
    }
    pub fn acknowledge(&self, ack_token: AckToken) -> Result<(), Error<W::Error>> {
        match ack_token {
            AckToken::Connack(connack) => {
                self.connack.signal(connack);
            }
            AckToken::Pingresp => {
                self.pingresp.signal(());
            }
            AckToken::Disconnect(reason) => {
                self.disconnect.signal(reason);
            }
            AckToken::Suback(id) => {
                self.packet_id_signals[id as usize - 1].signal(RecvAckToken::Suback);
            }
            AckToken::Puback(id, reason_code) => {
                self.packet_id_signals[id as usize - 1].signal(RecvAckToken::Puback(reason_code));
            }
            AckToken::Pubrec(id, reason_code) => {
                self.packet_id_signals[id as usize - 1].signal(RecvAckToken::Pubrec(reason_code));
            }
            AckToken::Publish(id) => {
                if let Some(id) = id {
                    self.send_acks
                        .try_send(SendAckToken::Publish(id))
                        .ok()
                        .ok_or(ProtocolError::ExceededRecvConcurrency)?;
                }
            }
        }
        Ok(())
    }
    async fn send(&self, packet: &Packet<'_>) -> Result<(), Error<W::Error>> {
        self.writer.lock().await.send_packet(packet).await?;
        Ok(())
    }
    pub async fn connect(
        &self,
        connect_request: &ConnectRequest<'_>,
    ) -> Result<(), Error<W::Error>> {
        if self.connect_started.replace(true) {
            return Err(Error::ProtocolError(ProtocolError::Unsupported));
        }
        self.send(&Packet::Connect(ConnectPacket {
            proto_name: "MQTT",
            proto_version: 5,
            clean_start: false,
            will: None,
            password: connect_request.password.clone(),
            username: connect_request.username.clone(),
            keep_alive: connect_request.keepalive,
            client_id: connect_request.client_id,
        }))
        .await?;
        let connack = self.connack.wait().await;
        match connack {
            ReasonCode::Success => {}
            error => return Err(ProtocolError::ConnectFailed(error).into()),
        }
        Ok(())
    }
    pub fn allocate_packet_id(&self) -> Result<u16, Error<W::Error>> {
        Ok(self
            .free_packet_ids
            .borrow_mut()
            .pop()
            .ok_or(ProtocolError::ExceededSendConcurrency)?)
    }
    async fn wait_packet(&self, packet_id: u16) -> RecvAckToken {
        let result = self.packet_id_signals[packet_id as usize - 1].wait().await;
        self.free_packet_ids.borrow_mut().push(packet_id).unwrap();
        result
    }
    pub async fn subscribe(&self, filter: &str) -> Result<(), Error<W::Error>> {
        let packet_id = self.allocate_packet_id()?;
        self.send(&Packet::Subscribe(SubscribePacket {
            packet_id,
            properties: &[],
            topic_filters: &[TopicFilter {
                topic_filter: filter,
                max_qos: Qos::ExactlyOnce,
                non_local: false,
                retain_handling: RetainHandling::Send,
            }],
        }))
        .await?;
        match self.wait_packet(packet_id).await {
            RecvAckToken::Suback => {}
            _ => return Err(ProtocolError::Malformed.into()),
        }
        Ok(())
    }
    pub async fn publish(&self, publish: &PublishRequest<'_>) -> Result<(), Error<W::Error>> {
        assert_eq!(publish.qos, Qos::AtMostOnce);
        let packet_id = match publish.qos {
            Qos::AtMostOnce => None,
            Qos::AtLeastOnce | Qos::ExactlyOnce => Some(self.allocate_packet_id()?),
        };
        self.send(&Packet::Publish(PublishPacket {
            dup: false,
            qos: publish.qos,
            retain: publish.retain,
            topic: publish.topic,
            packet_id,
            properties: &[],
            payload: publish.payload,
        }))
        .await?;
        if let Some(packet_id) = packet_id {
            match self.wait_packet(packet_id).await {
                RecvAckToken::Puback(ReasonCode::Success) => {}
                RecvAckToken::Pubrec(ReasonCode::Success) => {}
                RecvAckToken::Pubrec(e) => return Err(ProtocolError::PublishFailed(e).into()),
                RecvAckToken::Puback(e) => return Err(ProtocolError::PublishFailed(e).into()),
                _ => return Err(ProtocolError::Malformed.into()),
            }
        }
        Ok(())
    }
    pub async fn ping(&self) -> Result<(), Error<W::Error>> {
        // Ping has no packet ID, so just run it sequentially for simplicity.
        let lock = self.ping_mutex.lock().await;
        self.send(&Packet::Pingreq(PingreqPacket {})).await?;
        self.pingresp.wait().await;
        Ok(())
    }
    pub async fn send_acks(&self) -> Result<!, Error<W::Error>> {
        loop {
            match self.send_acks.receive().await {
                SendAckToken::Publish(u16) => {
                    todo!();
                }
            }
        }
    }
    pub async fn wait_disconnect(&self) -> Result<ReasonCode, Error<W::Error>> {
        Ok(self.disconnect.wait().await)
    }
}

#[derive(Clone, Debug)]
pub struct ConnectRequest<'a> {
    pub client_id: &'a str,
    pub username: Option<&'a str>,
    pub password: Option<&'a str>,
    pub keepalive: u16,
}

#[derive(Clone, Debug)]
pub struct PublishRequest<'a> {
    pub qos: Qos,
    pub topic: &'a str,
    pub payload: &'a [u8],
    pub retain: bool,
}

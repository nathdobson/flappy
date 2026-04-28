use crate::error::Error;
use crate::reader::MqttReader;
use crate::writer::MqttWriter;
use arena::Arena;
use core::cell::{Cell, RefCell};
use core::pin::pin;
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex;
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, RawMutex};
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embedded_io_async::{ErrorType, Read, Write};
use heapless::{Vec, VecView};
use log::warn;
use mqtt_core::error::ProtocolError;
use mqtt_core::protocol::{
    ConnectPacket, Packet, PingreqPacket, PublishPacket, Qos, ReasonCode, RetainHandling,
    SubscribePacket, TopicFilter,
};

pub struct MqttClient<
    M: RawMutex,
    W,
    R,
    const SEND_CAP: usize,
    const RECV_CONC: usize,
    const SEND_CONC: usize,
> {
    writer: Mutex<M, MqttWriter<W, SEND_CAP>>,
    connect_started: Cell<bool>,
    connack: Signal<M, ReasonCode>,
    ping_mutex: Mutex<M, ()>,
    pingresp: Signal<M, ()>,
    send_acks: Channel<M, SendAckToken, RECV_CONC>,
    packet_id_signals: [Signal<M, RecvAckToken>; SEND_CONC],
    free_packet_ids: blocking_mutex::Mutex<M, RefCell<Vec<u16, SEND_CONC>>>,
    disconnect: Signal<M, ReasonCode>,
    reader: Mutex<M, RefCell<MqttReader<R>>>,
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

impl<
    M: RawMutex,
    W: Write,
    R: Read,
    const SEND_CAP: usize,
    const RECV_CONC: usize,
    const SEND_CONC: usize,
> MqttClient<M, W, R, SEND_CAP, RECV_CONC, SEND_CONC>
{
    pub fn new(write: W, read: R) -> Self {
        let mut free_packet_ids = Vec::new();
        for x in 1u16..=SEND_CONC as u16 {
            free_packet_ids.push(x).unwrap();
        }
        MqttClient {
            writer: Mutex::new(MqttWriter::new(write)),
            connect_started: Cell::new(false),
            connack: Signal::new(),
            ping_mutex: Mutex::new(()),
            pingresp: Signal::new(),
            send_acks: Channel::new(),
            packet_id_signals: [const { Signal::new() }; SEND_CONC],
            free_packet_ids: blocking_mutex::Mutex::new(RefCell::new(free_packet_ids)),
            disconnect: Signal::new(),
            reader: Mutex::new(RefCell::new(MqttReader::new(read))),
        }
    }
    async fn send_acks(&self) -> Result<!, Error<W::Error, R::Error>> {
        loop {
            match self.send_acks.receive().await {
                SendAckToken::Publish(u16) => {
                    todo!();
                }
            }
        }
    }
    async fn wait_disconnect(&self) -> Result<ReasonCode, Error<W::Error, R::Error>> {
        Ok(self.disconnect.wait().await)
    }
    pub async fn run(&self) -> Result<!, Error<W::Error, R::Error>> {
        match select(self.wait_disconnect(), self.send_acks()).await {
            Either::First(x) => Err(ProtocolError::Disconnected(x?).into()),
            Either::Second(x) => x?,
        }
    }
    pub async fn receive<'ar>(
        &self,
        arena: &'ar Arena,
    ) -> Result<(AckToken, Packet<'ar>), Error<W::Error, R::Error>> {
        let reader = self.reader.lock().await;
        let mut reader = reader.borrow_mut();
        let packet = reader
            .read_packet(arena)
            .await
            .map_err(|e| e.map_write(|x| x))?;
        let token = match &packet {
            Packet::Connack(connack) => AckToken::Connack(connack.reason_code),
            Packet::Connect(_) => return Err(ProtocolError::Malformed.into()),
            Packet::Disconnect(disconnect) => AckToken::Disconnect(disconnect.reason),
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
    pub fn acknowledge(&self, ack_token: AckToken) -> Result<(), Error<W::Error, R::Error>> {
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
    pub async fn receive_with<U>(
        &self,
        arena: &mut [u8],
        mut handler: impl AsyncFnMut(&PublishPacket) -> Result<(), U>,
    ) -> Result<Result<!, U>, Error<W::Error, R::Error>> {
        loop {
            let mut arena = Arena::new(arena)?;
            let (ack, packet) = self.receive(arena).await?;
            match packet {
                Packet::Publish(publish) => {
                    if let Err(e) = handler(&publish).await {
                        return Ok(Err(e));
                    }
                }
                Packet::Disconnect(disconnect) => {
                    warn!("MQTT disconnected: {}", disconnect.reason);
                }
                _ => {}
            }
            self.acknowledge(ack)?;
        }
    }
    async fn send(&self, packet: &Packet<'_>) -> Result<(), Error<W::Error, R::Error>> {
        self.writer
            .lock()
            .await
            .send_packet(packet)
            .await
            .map_err(|e| e.map_read(|x| x))?;
        Ok(())
    }
    pub async fn connect(
        &self,
        connect_request: &ConnectRequest<'_>,
    ) -> Result<(), Error<W::Error, R::Error>> {
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
    pub fn allocate_packet_id(&self) -> Result<u16, Error<W::Error, R::Error>> {
        Ok(self.free_packet_ids.lock(|free_packet_ids| {
            Ok::<u16, Error<W::Error, R::Error>>(
                free_packet_ids
                    .borrow_mut()
                    .pop()
                    .ok_or(ProtocolError::ExceededSendConcurrency)?,
            )
        })?)
    }
    async fn wait_packet(&self, packet_id: u16) -> RecvAckToken {
        let result = self.packet_id_signals[packet_id as usize - 1].wait().await;
        self.free_packet_ids.lock(|free_packet_ids| {
            free_packet_ids.borrow_mut().push(packet_id).unwrap();
        });

        result
    }
    pub async fn subscribe(&self, filter: &str) -> Result<(), Error<W::Error, R::Error>> {
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
    pub async fn publish(
        &self,
        publish: &PublishRequest<'_>,
    ) -> Result<(), Error<W::Error, R::Error>> {
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
    pub async fn ping(&self) -> Result<(), Error<W::Error, R::Error>> {
        // Ping has no packet ID. It could be pipelined, but a mutex is simpler.
        let lock = self.ping_mutex.lock().await;
        self.send(&Packet::Pingreq(PingreqPacket {})).await?;
        self.pingresp.wait().await;
        Ok(())
    }
    pub async fn ping_keepalive<U>(
        &self,
        delay: impl AsyncFn() -> Result<(), U>,
    ) -> Result<Result<!, U>, Error<W::Error, R::Error>> {
        if let Err(e) = delay().await {
            return Ok(Err(e));
        }
        loop {
            let mut timer = pin!(delay());
            match select(&mut timer, self.ping()).await {
                Either::First(Ok(())) => return Err(ProtocolError::DeadlineExceeded.into()),
                Either::First(Err(e)) => return Ok(Err(e)),
                Either::Second(p) => p?,
            }
            if let Err(e) = timer.await {
                return Ok(Err(e));
            }
        }
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

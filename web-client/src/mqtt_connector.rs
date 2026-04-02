use crate::error::Error;
use crate::query_params::{QueryParams, QueryParamsCell};
use crate::status::{Status, StatusPriority};
use crate::utils::{sleep, try_window};
use arena::Arena;
use embassy_futures::select::{select, select5, Either, Either5};
use io_adapters::split::split_io;
use io_adapters::tokio::TokioStreamAdapter;
use log::{error, info};
use mqtt_client::receiver::MqttReceiver;
use mqtt_client::sender::{ConnectRequest, MqttSender, PublishRequest};
use mqtt_core::protocol::{Packet, Qos};
use protocol::display::{DisplayRequest, DisplayResponse};
use protocol::setup::DeviceInfo;
use serde::{Deserialize, Serialize};
use std::pin::pin;
use std::rc::Rc;
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;
use ws_stream_wasm::WsMeta;

const KEEPALIVE: u16 = 60;

pub struct PeekReceiver<T> {
    receiver: Receiver<T>,
    buffer: Option<T>,
}

pub enum DisplayResponseContainer {
    DisplayResponse(DisplayResponse),
    DeviceInfo(DeviceInfo),
}

impl<T> PeekReceiver<T> {
    pub fn new(receiver: Receiver<T>) -> Self {
        PeekReceiver {
            receiver,
            buffer: None,
        }
    }
    pub async fn recv(&mut self) -> Option<T> {
        if let Some(buffer) = self.buffer.take() {
            return Some(buffer);
        }
        self.receiver.recv().await
    }
    pub async fn peek_recv(&mut self) -> Option<&T> {
        if self.buffer.is_none() {
            self.buffer = self.receiver.recv().await;
        }
        self.buffer.as_ref()
    }
}

pub async fn run_mqtt(
    params: Rc<QueryParamsCell>,
    status: Rc<Status>,
    requests: Receiver<DisplayRequest>,
    mut responses: Sender<DisplayResponseContainer>,
) -> Result<!, Error> {
    let mut requests = PeekReceiver::new(requests);
    loop {
        let e = run_mqtt_once(
            params.clone(),
            status.clone(),
            &mut requests,
            &mut responses,
        )
        .await
        .into_err();
        error!("MQTT Connection failure: {}", e);
        status.set(StatusPriority::Error, format!("{}", e));
        requests.peek_recv().await.ok_or(Error::ChannelClosed)?;
    }
}
pub async fn run_mqtt_once(
    params: Rc<QueryParamsCell>,
    status: Rc<Status>,
    mut requests: &mut PeekReceiver<DisplayRequest>,
    responses: &mut Sender<DisplayResponseContainer>,
) -> Result<!, Error> {
    status.set(
        StatusPriority::Info,
        format!("Connecting to WebSocket {}", params.borrow().ws_url),
    );
    let (meta, stream) = WsMeta::connect(&params.borrow().ws_url, Some(vec!["mqtt"])).await?;
    let (read, write) = split_io(stream.into_io());
    let sender = MqttSender::<_, 1024, 1, 1>::new(TokioStreamAdapter(write));
    let mut receiver = MqttReceiver::new(TokioStreamAdapter(read));
    let req_topic = format!("{}/request", params.borrow().topic);
    let resp_topic = format!("{}/response", params.borrow().topic);
    let info_topic = format!("{}/info", params.borrow().topic);
    match select5(
        async {
            let mut arena_slice = [0u8; 1024];
            loop {
                let mut arena = Arena::new(&mut arena_slice).map_err(|_| Error::CapacityError)?;
                let (ack, packet) = receiver.receive(arena).await?;
                match packet {
                    Packet::Publish(publish) => {
                        if publish.topic == resp_topic {
                            match serde_json_core::from_slice::<DisplayResponse>(&publish.payload) {
                                Ok((response, _)) => {
                                    responses
                                        .send(DisplayResponseContainer::DisplayResponse(response))
                                        .await?
                                }
                                Err(e) => {
                                    error!("Could not parse message: {:?}", e);
                                }
                            }
                        } else if publish.topic == info_topic {
                            match serde_json_core::from_slice::<DeviceInfo>(&publish.payload) {
                                Ok((info, _)) => {
                                    responses
                                        .send(DisplayResponseContainer::DeviceInfo(info))
                                        .await?
                                }
                                Err(e) => {
                                    error!("Could not parse message: {:?}", e);
                                }
                            }
                        }
                    }
                    _ => {}
                }
                sender.acknowledge(ack)?;
            }
            Ok::<!, Error>(unreachable!())
        },
        async {
            sender.send_acks().await?;
            Ok::<!, Error>(unreachable!())
        },
        async {
            let disconnect = sender.wait_disconnect().await?;
            Err(Error::Disconnect(disconnect))
        },
        async {
            sleep(KEEPALIVE as i32 * 1000).await;
            loop {
                let mut timer = pin!(sleep(KEEPALIVE as i32 * 1000));
                match select(&mut timer, sender.ping()).await {
                    Either::First(()) => return Err(Error::DeadlineExceeded),
                    Either::Second(p) => p?,
                }
                timer.await
            }
            Ok::<!, Error>(unreachable!())
        },
        async {
            let client_id = format!("flappy_web_{}", Uuid::new_v4());
            status.set(
                StatusPriority::Info,
                format!(
                    "Connecting to MQTT with client_id `{}`, username `{}`, and password `{}`",
                    client_id,
                    params.borrow().username,
                    params.borrow().password
                ),
            );
            sender
                .connect(&ConnectRequest {
                    client_id: &client_id,
                    username: Some(&params.borrow().username),
                    password: Some(&params.borrow().password),
                    keepalive: KEEPALIVE,
                })
                .await?;
            status.set(
                StatusPriority::Info,
                format!("Subscribing to topic {}", resp_topic),
            );
            sender.subscribe(&resp_topic).await?;
            status.set(
                StatusPriority::Info,
                format!("Subscribing to topic {}", info_topic),
            );
            sender.subscribe(&info_topic).await?;
            status.set(StatusPriority::Info, "Waiting for Device Info".to_string());
            while let Some(next) = requests.recv().await {
                status.set(StatusPriority::Info, format!("Publishing `{:?}`", next));
                sender
                    .publish(&PublishRequest {
                        qos: Qos::AtMostOnce,
                        topic: &req_topic,
                        payload: &serde_json_core::to_vec::<DisplayRequest, 1024>(&next)?,
                        retain: false,
                    })
                    .await?;
                match next {
                    DisplayRequest::Run(msg) => {
                        status.set(StatusPriority::Info, format!("Sent \"{}\" to display.", msg));
                    }
                    DisplayRequest::RunSpindle(_) => {}
                    DisplayRequest::Test => {}
                }

            }
            Err(Error::ChannelClosed)
        },
    )
    .await
    {
        Either5::First(x) => x?,
        Either5::Second(x) => x?,
        Either5::Third(x) => x?,
        Either5::Fourth(x) => x?,
        Either5::Fifth(x) => x?,
    }
}

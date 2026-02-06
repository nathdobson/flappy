use crate::error::Error;
use crate::query_params::FlappyQueryParams;
use crate::status::{Status, StatusPriority};
use crate::utils::{sleep, try_window};
use crate::DisplayResponseContainer;
use arena::ArenaStorage;
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

pub async fn run_mqtt(
    params: FlappyQueryParams,
    status: Rc<Status>,
    mut requests: Receiver<DisplayRequest>,
    responses: Sender<DisplayResponseContainer>,
) -> Result<!, Error> {
    status.set(
        StatusPriority::Info,
        format!("Connecting to WebSocket {}", params.ws_url),
    );
    let (meta, stream) = WsMeta::connect(&params.ws_url, Some(vec!["mqtt"])).await?;
    let (read, write) = split_io(stream.into_io());
    let sender = MqttSender::<_, 1024, 1, 1>::new(TokioStreamAdapter(write));
    let mut receiver = MqttReceiver::new(TokioStreamAdapter(read));
    let req_topic = format!("{}/request", params.topic);
    let resp_topic = format!("{}/response", params.topic);
    let info_topic = format!("{}/info", params.topic);
    match select5(
        async {
            let mut arena_slice = [0u8; 1024];
            let mut arena = ArenaStorage::new(&mut arena_slice);
            loop {
                let (ack, packet) = receiver.receive(arena.start()).await?;
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
                    client_id, params.username, params.password
                ),
            );
            sender
                .connect(&ConnectRequest {
                    client_id: &client_id,
                    username: Some(&params.username),
                    password: Some(&params.password),
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
                status.set(StatusPriority::Info, format!("Published `{:?}`", next));
            }
            Ok::<!, Error>(unreachable!())
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

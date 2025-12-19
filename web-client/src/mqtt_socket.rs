use crate::error::Error;
use crate::utils::{sleep, try_window};
use arena::ArenaStorage;
use embassy_futures::select::{select, select5, Either, Either5};
use io_adapter::split::split_io;
use io_adapter::tokio::TokioStreamAdapter;
use log::{error, info};
use mqtt::proto::{Packet, Qos};
use mqtt::receiver::MqttReceiver;
use mqtt::sender::{ConnectRequest, MqttSender, PublishRequest};
use serde::{Deserialize, Serialize};
use std::pin::pin;
use tokio::sync::mpsc::{Receiver, Sender};
use ws_stream_wasm::WsMeta;
use proto::display::{DisplayMessage, DisplayRequest, DisplayResponse};

const KEEPALIVE: u16 = 60;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlappyQueryParams {
    ws_url: String,
    username: String,
    password: String,
    topic: String,
}

pub async fn run_mqtt(
    mut requests: Receiver<DisplayRequest>,
    responses: Sender<DisplayResponse>,
) -> Result<!, Error> {
    let search = try_window()?.location().search()?;
    let search = search.strip_prefix("?").unwrap_or(&search);
    let params: FlappyQueryParams = serde_qs::from_str(&search)?;
    let (meta, stream) = WsMeta::connect(&params.ws_url, Some(vec!["mqtt"])).await?;
    let (read, write) = split_io(stream.into_io());
    let sender = MqttSender::<_, 1024, 1, 1>::new(TokioStreamAdapter(write));
    let mut receiver = MqttReceiver::new(TokioStreamAdapter(read));
    match select5(
        async {
            let mut arena = ArenaStorage::<1024>::new();
            loop {
                let (ack, packet) = receiver.receive(arena.start()).await?;
                match packet {
                    Packet::Publish(publish) => {
                        match serde_json_core::from_slice::<DisplayMessage>(&publish.payload) {
                            Ok((m, _)) => match m {
                                DisplayMessage::Request(_) => {}
                                DisplayMessage::Response(response) => {
                                    responses.send(response).await?;
                                }
                            },
                            Err(e) => {
                                error!("Could not parse message: {:?}", e);
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
            let client_id = "flappy_web";
            info!(
                "Connecting to broker with client_id '{}' and username '{}'",
                client_id, params.username
            );
            sender
                .connect(&ConnectRequest {
                    client_id,
                    username: Some(&params.username),
                    password: Some(&params.password),
                    keepalive: 0,
                })
                .await?;
            info!("Connected to broker");
            info!("Subscribing to {}", params.topic);
            sender.subscribe(&params.topic).await?;
            info!("Subscribed");
            while let Some(next) = requests.recv().await {
                info!("Publishing {:?}", next);
                sender
                    .publish(&PublishRequest {
                        qos: Qos::AtMostOnce,
                        topic: &params.topic,
                        payload: &serde_json_core::to_vec::<_, 1024>(&DisplayMessage::Request(
                            next,
                        ))?,
                    })
                    .await?;
                info!("Published");
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

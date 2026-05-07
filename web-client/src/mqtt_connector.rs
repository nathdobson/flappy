use crate::error::Error;
use crate::query_params::QueryParamsCell;
use crate::status::{Status, StatusPriority};
use crate::utils::sleep;
use async_io_stream::IoStream;
use embassy_futures::select::{select4, Either4};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use io_adapters::split::{split_io, SplitRead, SplitWrite};
use io_adapters::tokio::TokioStreamAdapter;
use log::error;
use mqtt_client::client::{ConnectRequest, MqttClient, PublishRequest};
use mqtt_core::protocol::{PublishPacket, Qos};
use protocol::display::{DisplayRequest, DisplayResponse};
use protocol::setup::{DeviceInfo, MAX_SETUP_MESSAGE_SIZE};
use std::rc::Rc;
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;
use ws_stream_wasm::{WsMeta, WsStreamIo};

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

type FlappyMqttClient = MqttClient<
    NoopRawMutex,
    TokioStreamAdapter<SplitWrite<IoStream<WsStreamIo, Vec<u8>>>>,
    TokioStreamAdapter<SplitRead<IoStream<WsStreamIo, Vec<u8>>>>,
    1024,
    1,
    1,
>;

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
        status.set_error(StatusPriority::Error,"MQTT Connection failure", &e);
        requests.peek_recv().await.ok_or(Error::ChannelClosed)?;
    }
}

async fn handle_publish(
    publish: &PublishPacket<'_>,
    resp_topic: &str,
    info_topic: &str,
    responses: &mut Sender<DisplayResponseContainer>,
) -> Result<(), Error> {
    let mut tmp = vec![0; MAX_SETUP_MESSAGE_SIZE];
    if publish.topic == resp_topic {
        match serde_json_core::from_slice_escaped::<DisplayResponse>(&publish.payload, &mut tmp) {
            Ok((response, _)) => responses
                .send(DisplayResponseContainer::DisplayResponse(response))
                .await
                .map_err(|_| Error::SendError)?,
            Err(e) => {
                error!("Could not parse message: {:?}", e);
            }
        }
    } else if publish.topic == info_topic {
        match serde_json_core::from_slice_escaped::<DeviceInfo>(&publish.payload, &mut tmp) {
            Ok((info, _)) => responses
                .send(DisplayResponseContainer::DeviceInfo(info))
                .await
                .map_err(|_| Error::SendError)?,
            Err(e) => {
                error!("Could not parse message: {:?}", e);
            }
        }
    }
    Ok(())
}

async fn do_connect(
    client: &FlappyMqttClient,
    status: Rc<Status>,
    params: Rc<QueryParamsCell>,
    req_topic: &str,
    resp_topic: &str,
    info_topic: &str,
    requests: &mut PeekReceiver<DisplayRequest>,
) -> Result<!, Error> {
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
    client
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
    client.subscribe(&resp_topic).await?;
    status.set(
        StatusPriority::Info,
        format!("Subscribing to topic {}", info_topic),
    );
    client.subscribe(&info_topic).await?;
    status.set(StatusPriority::Info, "Waiting for Device Info".to_string());
    while let Some(next) = requests.recv().await {
        status.set(StatusPriority::Info, format!("Publishing `{:?}`", next));
        client
            .publish(&PublishRequest {
                qos: Qos::AtMostOnce,
                topic: &req_topic,
                payload: &serde_json_core::to_vec::<DisplayRequest, 1024>(&next)?,
                retain: false,
            })
            .await?;
        match next {
            DisplayRequest::Run(msg) => {
                status.set(
                    StatusPriority::Info,
                    format!("Sent \"{}\" to display.", msg),
                );
            }
            DisplayRequest::RunSpindle(_) => {}
            DisplayRequest::Test => {}
        }
    }
    Err(Error::ChannelClosed)
}

pub async fn run_mqtt_once(
    params: Rc<QueryParamsCell>,
    status: Rc<Status>,
    requests: &mut PeekReceiver<DisplayRequest>,
    responses: &mut Sender<DisplayResponseContainer>,
) -> Result<!, Error> {
    status.set(
        StatusPriority::Info,
        format!("Connecting to WebSocket {}", params.borrow().ws_url),
    );
    let (_meta, stream) = WsMeta::connect(&params.borrow().ws_url, Some(vec!["mqtt"])).await?;
    let (read, write) = split_io(stream.into_io());
    let client = FlappyMqttClient::new(TokioStreamAdapter(write), TokioStreamAdapter(read));
    let req_topic = format!("{}/request", params.borrow().topic);
    let resp_topic = format!("{}/response", params.borrow().topic);
    let info_topic = format!("{}/info", params.borrow().topic);
    match select4(
        client.run(),
        client.ping_keepalive(async || {
            sleep(KEEPALIVE as i32 * 1000).await;
            Ok::<(), Error>(())
        }),
        client.receive_with(&mut vec![0u8; 1024], async |publish| {
            handle_publish(publish, &resp_topic, &info_topic, responses).await?;
            Ok::<(), Error>(())
        }),
        do_connect(
            &client,
            status,
            params,
            &req_topic,
            &resp_topic,
            &info_topic,
            requests,
        ),
    )
    .await
    {
        Either4::First(x) => x?,
        Either4::Second(x) => x??,
        Either4::Third(x) => x??,
        Either4::Fourth(x) => x?,
    }
}

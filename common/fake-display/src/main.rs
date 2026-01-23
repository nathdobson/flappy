#![deny(unused_must_use)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![allow(unused_variables)]

use arena::ArenaStorage;
use clap::Parser;
use clap::builder::TypedValueParser;
use embassy_futures::select::select4;
use embassy_futures::select::{Either4, Either5, select5};
use glyph_render::Renderer;
use io_adapters::split::split_io;
use io_adapters::tokio::TokioStreamAdapter;
use mqtt_client::receiver::MqttReceiver;
use mqtt_client::sender::{ConnectRequest, MqttSender, PublishRequest};
use mqtt_core::protocol::{Packet, Qos};
use protocol::display::MAX_GLYPH_BYTES;
use protocol::display::MAX_GLYPHS;
use protocol::display::{DisplayRequest, DisplayResponse};
use protocol::setup::DeviceInfo;
use rustls::pki_types::ServerName;
use serde_json_core::heapless;
use serde_json_core::heapless::CapacityError;
use std::env;
use std::future::pending;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    mqtt_host: String,
    #[arg(long)]
    mqtt_port: u16,
    #[arg(long)]
    mqtt_client_id: String,
    #[arg(long)]
    mqtt_username: String,
    #[arg(long)]
    mqtt_password: String,
    #[arg(long)]
    mqtt_topic: String,
    #[arg(long)]
    glyph_count: usize,
    #[arg(long)]
    background_color: String,
    #[arg(long)]
    foreground_color: String,
    #[arg(long)]
    glyphs: Vec<String>,
}

const KEEPALIVE: u16 = 60;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut root_cert_store = rustls::RootCertStore::empty();
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));
    let stream = TcpStream::connect((args.mqtt_host.clone(), args.mqtt_port))
        .await
        .unwrap();
    let stream = connector
        .connect(ServerName::try_from(args.mqtt_host).unwrap(), stream)
        .await
        .unwrap();
    let (read, write) = split_io(stream);
    let sender = MqttSender::<_, 1024, 1, 1>::new(TokioStreamAdapter(write));
    let mut receiver = MqttReceiver::new(TokioStreamAdapter(read));
    let (request_send, mut request_recv) = tokio::sync::mpsc::unbounded_channel::<DisplayRequest>();
    let req_topic = format!("{}/request", args.mqtt_topic);
    let resp_topic = format!("{}/response", args.mqtt_topic);
    let info_topic = format!("{}/info", args.mqtt_topic);
    match select5(
        async {
            println!("Connecting...");
            sender
                .connect(&ConnectRequest {
                    client_id: "sfasfgasfgf",
                    username: Some(&args.mqtt_username),
                    password: Some(&args.mqtt_password),
                    keepalive: KEEPALIVE,
                })
                .await?;
            println!("Subscribing...");
            sender.subscribe(&req_topic).await?;
            println!("Publishing Device Info...");
            let device_info: heapless::Vec<u8, 1024> = serde_json_core::to_vec(&DeviceInfo {
                serial: 0,
                git_version: Default::default(),
                git_dirty: None,
                git_head_ref: Default::default(),
                glyphs: args.glyph_count,
                background: (*args.background_color).try_into()?,
                foreground: (*args.foreground_color).try_into()?,
            })?;
            sender
                .publish(&PublishRequest {
                    qos: Qos::AtMostOnce,
                    topic: &info_topic,
                    payload: &device_info,
                    retain: true,
                })
                .await?;
            println!("Ready");
            pending::<()>().await;
            anyhow::Result::<()>::Ok(())
        },
        async {
            let mut tmp = [0u8; 1024];
            let mut arena = ArenaStorage::<1024>::new();
            loop {
                let (token, packet): (_, Packet) = receiver.receive(arena.start()).await?;
                match packet {
                    Packet::Publish(packet) => {
                        let request = serde_json_core::from_slice_escaped::<DisplayRequest>(
                            &packet.payload,
                            &mut tmp,
                        )?
                        .0;
                        request_send.send(request)?;
                    }
                    _ => {}
                }
                sender.acknowledge(token)?;
            }
            anyhow::Result::<()>::Ok(())
        },
        async {
            sender.send_acks().await?;
            anyhow::Result::<()>::Ok(())
        },
        async {
            loop {
                tokio::time::sleep(Duration::from_secs(KEEPALIVE as u64)).await;
                sender.ping().await?;
            }
            anyhow::Result::<()>::Ok(())
        },
        async {
            while let Some(next) = request_recv.recv().await {
                match next {
                    DisplayRequest::Run(a) => {
                        let glyphs = args.glyphs.iter().map(|x| &**x).collect::<Vec<_>>();
                        let mut renderer = Renderer::<MAX_GLYPHS>::new(&glyphs);
                        renderer.append(&a)?;
                        let rendered = renderer.finish();
                        let rendered = rendered
                            .iter()
                            .map(|x| {
                                heapless::String::<MAX_GLYPH_BYTES>::try_from(&*args.glyphs[*x])
                            })
                            .collect::<Result<
                                heapless::Vec<heapless::String<MAX_GLYPH_BYTES>, MAX_GLYPHS>,
                                CapacityError,
                            >>()?;
                        println!("Rendered = {:?}", rendered);
                        sender
                            .publish(&PublishRequest {
                                qos: Qos::AtMostOnce,
                                topic: &resp_topic,
                                payload: &serde_json_core::to_vec::<_, 1024>(
                                    &DisplayResponse::Start(rendered.clone()),
                                )?,
                                retain: false,
                            })
                            .await?;
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        sender
                            .publish(&PublishRequest {
                                qos: Qos::AtMostOnce,
                                topic: &resp_topic,
                                payload: &serde_json_core::to_vec::<_, 1024>(
                                    &DisplayResponse::Stop(rendered),
                                )?,
                                retain: true,
                            })
                            .await?;
                    }
                    DisplayRequest::Test => {}
                }
            }
            anyhow::Result::<()>::Ok(())
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
    Ok(())
}

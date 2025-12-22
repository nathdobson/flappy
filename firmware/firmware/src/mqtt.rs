use crate::error::Error;
use arena::ArenaStorage;
use core::cell::Cell;
use core::fmt;
use core::fmt::{Display, Formatter, write};
use core::future::pending;
use core::intrinsics::unreachable;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, Either4, Either5, select, select4, select5};
use embassy_futures::yield_now;
use embassy_net::dns;
use embassy_net::dns::DnsQueryType;
use embassy_net::tcp::{TcpReader, TcpSocket, TcpWriter};
use embassy_rp::clocks::RoscRng;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_sync::watch::{DynReceiver, Watch};
use embassy_time::{Duration, TimeoutError, Timer, with_timeout};
use embedded_io_async::{ErrorType, Read, Write};
use embedded_tls::{
    Aes128GcmSha256, Certificate, NoVerify, SplitConnectionState, TlsConfig, TlsConnection,
    TlsContext, TlsError, TlsVerifier, TlsWriter,
};
use log::{error, info, warn};
use mqtt::proto::{Packet, PublishPacket, Qos};
use mqtt::receiver::MqttReceiver;
use mqtt::sender::{ConnectRequest, MqttSender, PublishRequest};
use serde::{Deserialize, Serialize};
use smoltcp::wire::IpEndpoint;
use static_cell::make_static;
use trouble_host::prelude::HeaplessString;
// use rust_mqtt::client::client::MqttClient;
// use rust_mqtt::client::client_config::ClientConfig;
// use rust_mqtt::packet::v5::reason_codes::ReasonCode;
// use rust_mqtt::utils::rng_generator::CountingRng;
use proto::display::{DisplayMessage, DisplayResponse};

const MODULE: &'static str = "[MQTT ]";
const KEEPALIVE: u16 = 60;
const PACKET_SIZE: usize = 1024;

use proto::display::DisplayRequest;
use proto::setup::{AppSettings, MqttServiceError, MqttServiceStatus, MqttSettings};

pub struct MqttModule {
    spawner: Spawner,
    stack: &'static embassy_net::Stack<'static>,
    settings: Signal<NoopRawMutex, MqttSettings>,
    display_request: &'static Signal<NoopRawMutex, DisplayRequest>,
    display_response: &'static Signal<NoopRawMutex, DisplayResponse>,
    status: Watch<NoopRawMutex, MqttServiceStatus, 1>,
}

struct TcpSocketMutex<'a> {
    write: Mutex<NoopRawMutex, TcpWriter<'a>>,
    read: Mutex<NoopRawMutex, TcpReader<'a>>,
}

impl<'a, 'b> ErrorType for &'a TcpSocketMutex<'b> {
    type Error = embassy_net::tcp::Error;
}

impl<'a, 'b> Write for &'a TcpSocketMutex<'b> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.write.lock().await.write(buf).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.write.lock().await.flush().await
    }
}

impl<'a, 'b> Read for &'a TcpSocketMutex<'b> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.read.lock().await.read(buf).await
    }
}

impl MqttModule {
    pub async fn new(
        spawner: Spawner,
        stack: &'static embassy_net::Stack<'static>,
        display_request: &'static Signal<NoopRawMutex, DisplayRequest>,
        display_response: &'static Signal<NoopRawMutex, DisplayResponse>,
    ) -> Result<&'static MqttModule, Error> {
        let module = make_static!(MqttModule {
            spawner,
            stack,
            settings: Signal::new(),
            display_request,
            display_response,
            status: Watch::new(),
        });
        spawner.spawn({
            #[embassy_executor::task]
            async fn run_task(this: &'static MqttModule) {
                this.run().await;
            }
            run_task(module)?
        });

        Ok(module)
    }

    async fn run(&'static self) {
        let mut settings = self.settings.wait().await;
        let mut rx_buffer = make_static!([0; PACKET_SIZE]);
        let mut tx_buffer = make_static!([0; PACKET_SIZE]);
        loop {
            if let Some(s) = self.settings.try_take() {
                settings = s;
            }
            // Cancel the MQTT connection every time the settings change.
            match select(
                self.settings.wait(),
                self.run_with_settings(&settings, rx_buffer, tx_buffer),
            )
            .await
            {
                Either::First(s) => {
                    info!("{MODULE} Updated MQTT settings");
                    settings = s
                }
                Either::Second(Ok(x)) => match x {},
                Either::Second(Err(e)) => {
                    error!("{MODULE} error during MQTT loop: {:?}", e);
                    self.status.sender().send(MqttServiceStatus::Error(e));
                    Timer::after(Duration::from_secs(10)).await;
                }
            }
        }
    }

    async fn run_with_settings(
        &'static self,
        settings: &MqttSettings,

        rx_buffer: &mut [u8],
        tx_buffer: &mut [u8],
    ) -> Result<!, MqttServiceError> {
        if settings.hostname.is_empty() {
            self.status.sender().send(MqttServiceStatus::Unconfigured);
            pending::<!>().await;
        }
        info!(
            "{MODULE} [WiFi] Connecting to WiFi with MAC {}",
            self.stack.hardware_address()
        );
        self.status.sender().send(MqttServiceStatus::WaitingForLink);
        self.stack.wait_link_up().await;
        info!("{MODULE} [WiFi] Connecting to WiFi");
        info!("{MODULE} [WiFi] Waiting for IP address");
        self.status.sender().send(MqttServiceStatus::WaitingForDhcp);
        self.stack.wait_config_up().await;
        if let Some(config) = self.stack.config_v4() {
            info!(
                "{MODULE} [WiFi] Connected to IPv4 with {}",
                fmt::from_fn(|f| {
                    write!(f, "IP = {}, ", config.address)?;
                    for dns in &config.dns_servers {
                        write!(f, "DNS = {}, ", dns)?;
                    }
                    if let Some(gateway) = config.gateway {
                        write!(f, "GATEWAY = {}, ", gateway)?;
                    }
                    Ok(())
                })
            );
        }
        if let Some(config) = self.stack.config_v6() {
            info!(
                "{MODULE} [WiFi] Connected to IPv6 with {}",
                fmt::from_fn(|f| {
                    write!(f, "IP = {}, ", config.address)?;
                    for dns in &config.dns_servers {
                        write!(f, "DNS = {}, ", dns)?;
                    }
                    if let Some(gateway) = config.gateway {
                        write!(f, "GATEWAY = {}, ", gateway)?;
                    }
                    Ok(())
                })
            );
        }

        let mut socket = TcpSocket::new(*self.stack, rx_buffer, tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(60)));
        let dns = &*settings.hostname;
        let port = settings.port;
        info!("{MODULE} [DNS] Querying {:?}", dns);
        self.status.sender().send(MqttServiceStatus::DnsQuery);
        let addr = self
            .stack
            .dns_query(dns, DnsQueryType::A)
            .await
            .map_err(|e| MqttServiceError::DnsError)?[0];

        let remote_endpoint = IpEndpoint { addr, port };
        info!("{MODULE} [DNS] Resolved {}", remote_endpoint);
        self.status.sender().send(MqttServiceStatus::TcpConnect);

        info!("{MODULE} [TCP] Connecting to {}", remote_endpoint);
        socket
            .connect(remote_endpoint)
            .await
            .map_err(|e| MqttServiceError::TcpError)?;
        info!(
            "{MODULE} [TCP] Connected ({} -> {})",
            fmt::from_fn(|f| {
                if let Some(local) = socket.local_endpoint() {
                    write!(f, "{}", local)?
                }
                Ok(())
            }),
            fmt::from_fn(|f| {
                if let Some(remote) = socket.remote_endpoint() {
                    write!(f, "{}", remote)?
                }
                Ok(())
            }),
        );

        let (read, write) = socket.split();
        let socket_mutex = TcpSocketMutex {
            write: Mutex::new(write),
            read: Mutex::new(read),
        };

        let mut read_record_buffer = [0; 16384];
        let mut write_record_buffer = [0; 16384];
        let config = TlsConfig::<Aes128GcmSha256>::new()
            .with_server_name(dns)
            .enable_rsa_signatures();
        let mut tls = TlsConnection::new(
            &socket_mutex,
            &mut read_record_buffer,
            &mut write_record_buffer,
        );

        self.status.sender().send(MqttServiceStatus::TlsConnect);
        info!("{MODULE} [TLS] Starting handshake");
        tls.open::<_, NoVerify>(TlsContext::new(&config, &mut RoscRng))
            .await
            .map_err(|e| MqttServiceError::TlsError)?;
        info!("{MODULE} [TLS] Handshake complete");

        let mut state = SplitConnectionState::default();
        let (read, write): (_, TlsWriter<_, _, _>) = tls.split_with(&mut state);
        let sender = MqttSender::<_, 1024, 1, 1>::new(write);
        let mut receiver = MqttReceiver::new(read);
        match select5(
            async {
                let mut arena = ArenaStorage::<1024>::new();
                loop {
                    let (ack, packet) = receiver
                        .receive(arena.start())
                        .await
                        .map_err(|e| MqttServiceError::MqttError)?;
                    match packet {
                        Packet::Publish(publish) => {
                            self.handle_publish(&publish);
                        }
                        Packet::Disconnect(disconnect) => {
                            info!("Disconnected: {}", disconnect.reason);
                        }
                        _ => {}
                    }
                    sender
                        .acknowledge(ack)
                        .map_err(|e| MqttServiceError::MqttError)?;
                }
                Ok::<!, MqttServiceError>(unreachable!())
            },
            async {
                let disconnect = sender
                    .wait_disconnect()
                    .await
                    .map_err(|e| MqttServiceError::MqttError)?;
                error!("Disconnect: {}", disconnect);
                Err::<!, MqttServiceError>(MqttServiceError::Disconnected)
            },
            async {
                sender
                    .send_acks()
                    .await
                    .map_err(|e| MqttServiceError::MqttError)?;
                Ok::<!, MqttServiceError>(unreachable!())
            },
            async {
                Timer::after(Duration::from_secs(KEEPALIVE as u64)).await;
                loop {
                    let mut timer = Timer::after(Duration::from_secs(KEEPALIVE as u64));
                    match select(&mut timer, sender.ping()).await {
                        Either::First(()) => return Err(MqttServiceError::DeadlineExceeded),
                        Either::Second(p) => p.map_err(|_| MqttServiceError::MqttError)?,
                    }
                    timer.await
                }
                Ok::<!, MqttServiceError>(unreachable!())
            },
            async {
                self.status.sender().send(MqttServiceStatus::MqttConnect);
                info!(
                    "{MODULE} Connecting to broker with client_id '{}' and username '{}'",
                    proto::PRODUCT_NAME,
                    settings.username
                );
                sender
                    .connect(&ConnectRequest {
                        client_id: proto::PRODUCT_NAME,
                        username: Some(&settings.username),
                        password: Some(&settings.password),
                        keepalive: 0,
                    })
                    .await
                    .map_err(|e| MqttServiceError::MqttError)?;
                info!("{MODULE} Connected to broker");

                self.status.sender().send(MqttServiceStatus::MqttSubscribe);
                info!("{MODULE} Subscribing to {}", settings.topic);
                sender
                    .subscribe(&settings.topic)
                    .await
                    .map_err(|e| MqttServiceError::MqttError)?;
                info!("{MODULE} Subscribed");

                self.status.sender().send(MqttServiceStatus::Connected);
                loop {
                    let response = self.display_response.wait().await;
                    info!("Sending response: {:?}", response);
                    match serde_json_core::to_vec::<DisplayMessage, 128>(&DisplayMessage::Response(
                        response,
                    )) {
                        Ok(encoded) => {
                            let request = PublishRequest {
                                qos: Qos::AtMostOnce,
                                topic: &settings.topic,
                                payload: &encoded,
                            };
                            sender
                                .publish(&request)
                                .await
                                .map_err(|e| MqttServiceError::MqttError)?;
                            info!("Sent response");
                        }
                        Err(e) => {
                            warn!("Cannot encode response {:?}", e);
                        }
                    }
                }
                Ok::<!, MqttServiceError>(unreachable!())
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
    pub fn handle_publish(&self, publish: &PublishPacket<'_>) {
        let Ok(message) = str::from_utf8(publish.payload) else {
            warn!("{MODULE} Invalid UTF-8 payload");
            return;
        };
        info!("{MODULE} Received message on topic {}", publish.topic);
        info!("{MODULE} {:?}", message);
        for lines in message.lines() {
            info!("{MODULE}    {}", lines);
        }
        let message = match serde_json_core::from_str_escaped::<DisplayMessage>(
            message,
            &mut [0; PACKET_SIZE],
        ) {
            Ok((message, _)) => message,
            Err(e) => {
                error!("{MODULE} Failed to parse message: {:?}", e);
                return;
            }
        };
        info!("{MODULE} Parsed: {:?}", message);
        match message {
            DisplayMessage::Request(req) => self.display_request.signal(req),
            DisplayMessage::Response(_) => {}
        }
    }
    pub fn set_settings(&self, settings: MqttSettings) {
        info!("{MODULE} Updating mqtt settings");
        self.settings.signal(settings);
    }
    pub fn watch_status(&'static self) -> Option<DynReceiver<'static, MqttServiceStatus>> {
        self.status.dyn_receiver()
    }
}

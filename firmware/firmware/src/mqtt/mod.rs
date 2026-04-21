mod error;
mod merge_socket;
mod tls;
mod webpki_provider;

use crate::error::Error;
use arena::Arena;
use board_info::serial_number;
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
use embassy_net::tcp::{ConnectError, TcpReader, TcpSocket, TcpWriter};
use embassy_rp::clocks::RoscRng;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_sync::watch::{DynReceiver, Watch};
use embassy_time::{Duration, TimeoutError, Timer, with_timeout};
use embedded_io::ErrorKind;
use embedded_io_async::{ErrorType, Read, Write};
use embedded_tls::alert::{AlertDescription, AlertLevel};
use embedded_tls::webpki::CertVerifier;
use embedded_tls::{
    Aes128GcmSha256, Certificate, NoClock, NoVerify, TlsCipherSuite, TlsConfig, TlsConnection,
    TlsContext, TlsError, TlsVerifier, TlsWriter, UnsecureProvider,
};
use heapless::{String, format};
use log::{error, info, trace, warn};
use make_static::make_static;
use mqtt_client::receiver::MqttReceiver;
use mqtt_client::sender::{ConnectRequest, MqttSender, PublishRequest};
use mqtt_core::protocol::{Packet, PublishPacket, Qos};
use serde::{Deserialize, Serialize};
use smoltcp::wire::IpEndpoint;
// use rust_mqtt::client::client::MqttClient;
// use rust_mqtt::client::client_config::ClientConfig;
// use rust_mqtt::packet::v5::reason_codes::ReasonCode;
// use rust_mqtt::utils::rng_generator::CountingRng;
use protocol::display::DisplayResponse;

const MODULE: &'static str = "[MQTT ]";
const KEEPALIVE: u16 = 60;
const PACKET_SIZE: usize = 1024;
const RECORD_SIZE: usize = 16384;

use crate::application::DisplayResponseContainer;
use crate::mqtt::error::convert_mqtt_error;
use crate::mqtt::merge_socket::MergeSocket;
use crate::mqtt::tls::TlsConnectionBuilder;
use crate::mqtt::webpki_provider::WebPkiProvider;
use protocol::display::DisplayRequest;
use protocol::error::{
    DnsError, EmbeddedIoErrorKind, MqttServiceError, TcpError, TlsAlertDescription, TlsAlertLevel,
    TlsParseError,
};
use protocol::setup::{AppSettings, DeviceInfo, MqttServiceStatus, MqttSettings};
use protocol::{PRODUCT_NAME, PRODUCT_SHORT_NAME};
use runtime::LocalSpawn;

pub struct MqttModule {
    spawner: Spawner,
    stack: &'static embassy_net::Stack<'static>,
    settings: Signal<NoopRawMutex, MqttSettings>,
    display_request: &'static Signal<NoopRawMutex, DisplayRequest>,
    display_response: &'static Channel<NoopRawMutex, DisplayResponseContainer, 1>,
    status: Watch<NoopRawMutex, MqttServiceStatus, 1>,
}

impl MqttModule {
    pub fn new(
        spawner: Spawner,
        stack: &'static embassy_net::Stack<'static>,
        display_request: &'static Signal<NoopRawMutex, DisplayRequest>,
        display_response: &'static Channel<NoopRawMutex, DisplayResponseContainer, 1>,
    ) -> Result<&'static MqttModule, Error> {
        let module: &_ = make_static!(
            MqttModule,
            MqttModule {
                spawner,
                stack,
                settings: Signal::new(),
                display_request,
                display_response,
                status: Watch::new(),
            }
        );
        make_static!(_, LocalSpawn::new(spawner)).spawn(move || async move {
            module.run().await;
        });

        Ok(module)
    }

    async fn run(&'static self) {
        let mut settings = self.settings.wait().await;
        let mut rx_buffer = make_static!([u8; PACKET_SIZE], [0; PACKET_SIZE]);
        let mut tx_buffer = make_static!([u8; PACKET_SIZE], [0; PACKET_SIZE]);
        let mut read_record_buffer = make_static!([u8; RECORD_SIZE], [0; RECORD_SIZE]);
        let mut write_record_buffer = make_static!([u8; RECORD_SIZE], [0; RECORD_SIZE]);
        loop {
            if let Some(s) = self.settings.try_take() {
                settings = s;
            }
            // Cancel the MQTT connection every time the settings change.
            match select(
                self.settings.wait(),
                self.run_with_settings(
                    &settings,
                    rx_buffer,
                    tx_buffer,
                    read_record_buffer,
                    write_record_buffer,
                ),
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
        read_record_buffer: &mut [u8],
        write_record_buffer: &mut [u8],
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

        let mut tls = TlsConnectionBuilder {
            rx_buffer,
            tx_buffer,
            read_record_buffer,
            write_record_buffer,
            hostname: &settings.hostname,
            port: settings.port,
            stack: self.stack,
        };
        self.status.sender().send(MqttServiceStatus::DnsQuery);
        let mut tls = tls.resolve_dns().await?;
        self.status.sender().send(MqttServiceStatus::TcpConnect);
        let mut tls = tls.connect_tcp().await?;
        self.status.sender().send(MqttServiceStatus::TlsConnect);
        let mut tls = tls.merge_socket();
        let mut tls = tls.connect_tls().await?;

        let (read, write): (_, TlsWriter<_, _>) = tls.split();
        let sender = MqttSender::<_, 1024, 1, 1>::new(write);
        let mut receiver = MqttReceiver::new(read);
        match select5(
            async {
                let mut arena_slice = [0u8; 1024];
                loop {
                    let mut arena =
                        Arena::new(&mut arena_slice).map_err(|e| MqttServiceError::AllocError)?;
                    let (ack, packet) =
                        receiver.receive(arena).await.map_err(convert_mqtt_error)?;
                    match packet {
                        Packet::Publish(publish) => {
                            self.handle_publish(&publish);
                        }
                        Packet::Disconnect(disconnect) => {
                            info!("Disconnected: {}", disconnect.reason);
                        }
                        _ => {}
                    }
                    sender.acknowledge(ack).map_err(convert_mqtt_error)?;
                }
                Ok::<!, MqttServiceError>(unreachable!())
            },
            async {
                let disconnect = sender.wait_disconnect().await.map_err(convert_mqtt_error)?;
                error!("Disconnect: {}", disconnect);
                Err::<!, MqttServiceError>(MqttServiceError::Disconnected)
            },
            async {
                sender.send_acks().await.map_err(convert_mqtt_error)?;
                Ok::<!, MqttServiceError>(unreachable!())
            },
            async {
                Timer::after(Duration::from_secs(KEEPALIVE as u64)).await;
                loop {
                    let mut timer = Timer::after(Duration::from_secs(KEEPALIVE as u64));
                    match select(&mut timer, sender.ping()).await {
                        Either::First(()) => return Err(MqttServiceError::DeadlineExceeded),
                        Either::Second(p) => p.map_err(convert_mqtt_error)?,
                    }
                    timer.await
                }
                Ok::<!, MqttServiceError>(unreachable!())
            },
            async {
                self.status.sender().send(MqttServiceStatus::MqttConnect);
                info!(
                    "{MODULE} Connecting to broker with client_id '{:?}' and username '{}'",
                    settings.client_id, settings.username
                );
                sender
                    .connect(&ConnectRequest {
                        client_id: settings
                            .client_id
                            .as_deref()
                            .unwrap_or(serial_number().unwrap_or(PRODUCT_NAME)),
                        username: Some(&settings.username),
                        password: Some(&settings.password),
                        keepalive: 0,
                    })
                    .await
                    .map_err(convert_mqtt_error)?;
                info!("{MODULE} Connected to broker");

                self.status.sender().send(MqttServiceStatus::MqttSubscribe);
                let request_topic: String<128> = format!("{}/request", settings.topic)
                    .ok()
                    .ok_or(MqttServiceError::TopicTooLong)?;
                let response_topic: String<128> = format!("{}/response", settings.topic)
                    .ok()
                    .ok_or(MqttServiceError::TopicTooLong)?;
                let info_topic: String<128> = format!("{}/info", settings.topic)
                    .ok()
                    .ok_or(MqttServiceError::TopicTooLong)?;
                info!("{MODULE} Subscribing to {}", request_topic);
                sender
                    .subscribe(&request_topic)
                    .await
                    .map_err(convert_mqtt_error)?;
                info!("{MODULE} Subscribed");

                self.status.sender().send(MqttServiceStatus::Connected);
                loop {
                    let response = self.display_response.receive().await;
                    match response {
                        DisplayResponseContainer::DisplayResponse(response) => {
                            match serde_json_core::to_vec::<DisplayResponse, PACKET_SIZE>(&response)
                            {
                                Ok(response) => {
                                    sender
                                        .publish(&PublishRequest {
                                            qos: Qos::AtMostOnce,
                                            topic: &response_topic,
                                            payload: &response,
                                            retain: true,
                                        })
                                        .await
                                        .map_err(convert_mqtt_error)?;
                                }
                                Err(e) => {
                                    warn!("Cannot encode response {:?}", e);
                                }
                            }
                        }
                        DisplayResponseContainer::DeviceInfo(info) => {
                            match serde_json_core::to_vec::<DeviceInfo, PACKET_SIZE>(&info) {
                                Ok(info) => {
                                    sender
                                        .publish(&PublishRequest {
                                            qos: Qos::AtMostOnce,
                                            topic: &info_topic,
                                            payload: &info,
                                            retain: true,
                                        })
                                        .await
                                        .map_err(convert_mqtt_error)?;
                                }
                                Err(e) => {
                                    warn!("Cannot encode info {:?}", e);
                                }
                            }
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
        trace!("{MODULE} Received message on topic {}", publish.topic);
        trace!("{MODULE} {:?}", message);
        for lines in message.lines() {
            trace!("{MODULE}    {}", lines);
        }
        let request = match serde_json_core::from_str_escaped::<DisplayRequest>(
            message,
            &mut [0; PACKET_SIZE],
        ) {
            Ok((message, _)) => message,
            Err(e) => {
                error!("{MODULE} Failed to parse message: {:?}", e);
                return;
            }
        };
        trace!("{MODULE} Parsed message: {:?}", message);
        self.display_request.signal(request);
    }
    pub fn set_settings(&self, settings: MqttSettings) {
        info!("{MODULE} Updating mqtt settings");
        self.settings.signal(settings);
    }
    pub fn watch_status(&'static self) -> Option<DynReceiver<'static, MqttServiceStatus>> {
        self.status.dyn_receiver()
    }
}

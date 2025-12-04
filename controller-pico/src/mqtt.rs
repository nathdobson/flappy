use crate::error::Error;
use crate::product::PRODUCT_NAME;
use crate::wifi::WifiModule;
use core::cell::Cell;
use core::fmt;
use core::fmt::{Display, Formatter, write};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_futures::yield_now;
use embassy_net::dns::DnsQueryType;
use embassy_net::tcp::TcpSocket;
use embassy_rp::clocks::RoscRng;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, TimeoutError, Timer, with_timeout};
use embedded_tls::{Aes128GcmSha256, NoVerify, TlsConfig, TlsConnection, TlsContext};
use log::{error, info};
use rust_mqtt::client::client::MqttClient;
use rust_mqtt::client::client_config::ClientConfig;
use rust_mqtt::packet::v5::reason_codes::ReasonCode;
use rust_mqtt::utils::rng_generator::CountingRng;
use serde::{Deserialize, Serialize};
use smoltcp::wire::IpEndpoint;
use static_cell::make_static;
use trouble_host::prelude::HeaplessString;

const MODULE: &'static str = "[MQTT ]";
pub struct MqttModule {
    spawner: Spawner,
    stack: &'static embassy_net::Stack<'static>,
    signal: Signal<NoopRawMutex, MqttSettings>,
    started: Cell<bool>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct MqttSettings {
    pub hostname: HeaplessString<128>,
    pub port: u16,
    pub username: HeaplessString<128>,
    pub password: HeaplessString<128>,
    pub topic: HeaplessString<128>,
}

pub enum MqttStatus {
    Disconnected,
    Connected,
    WaitingForLink,
    WaitingForDhcp,
    DnsQuery,
    TcpConnect,
    TlsConnect,
    MqttConnect,
    MqttSubscribe,
    Error(Error),
}

pub trait MqttHandler {
    fn handle_status(&self, status: MqttStatus);
    fn handle(&self, topic: &str, message: &[u8]);
}

impl MqttModule {
    pub async fn new(
        spawner: Spawner,
        stack: &'static embassy_net::Stack<'static>,
    ) -> Result<&'static MqttModule, Error> {
        let module = make_static!(MqttModule {
            spawner,
            stack,
            signal: Signal::new(),
            started: Cell::new(false),
        });
        Ok(module)
    }

    pub fn start(&'static self, handler: &'static dyn MqttHandler) -> Result<(), Error> {
        assert!(!self.started.replace(true));
        self.spawner.spawn({
            #[embassy_executor::task]
            async fn run_task(this: &'static MqttModule, handler: &'static dyn MqttHandler) {
                this.run(handler).await;
            }
            run_task(self, handler)?
        });
        Ok(())
    }

    async fn run(&'static self, handler: &'static dyn MqttHandler) {
        let mut settings = self.signal.wait().await;
        loop {
            if let Some(s) = self.signal.try_take() {
                settings = s;
            }
            // Cancel the MQTT connection every time the settings change.
            match select(
                self.signal.wait(),
                self.run_with_settings(&settings, handler),
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
                    handler.handle_status(MqttStatus::Error(e));
                    Timer::after(Duration::from_secs(10)).await;
                }
            }
        }
    }

    async fn run_with_settings(
        &'static self,
        settings: &MqttSettings,
        handler: &'static dyn MqttHandler,
    ) -> Result<!, Error> {
        info!(
            "{MODULE} [WiFi] Connecting to WiFi with MAC {}",
            self.stack.hardware_address()
        );
        handler.handle_status(MqttStatus::WaitingForLink);
        self.stack.wait_link_up().await;
        info!("{MODULE} [WiFi] Connecting to WiFi");
        info!("{MODULE} [WiFi] Waiting for IP address");
        handler.handle_status(MqttStatus::WaitingForDhcp);
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

        let mut rx_buffer = [0; 4096];
        let mut tx_buffer = [0; 4096];
        let mut socket = TcpSocket::new(*self.stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(20)));
        let dns = &*settings.hostname;
        let port = settings.port;
        info!("{MODULE} [DNS] Querying {:?}", dns);
        handler.handle_status(MqttStatus::DnsQuery);
        let addr = self.stack.dns_query(dns, DnsQueryType::A).await?[0];

        let remote_endpoint = IpEndpoint { addr, port };
        info!("{MODULE} [DNS] Resolved {}", remote_endpoint);
        handler.handle_status(MqttStatus::TcpConnect);

        info!("{MODULE} [TCP] Connecting to {}", remote_endpoint);
        socket.connect(remote_endpoint).await?;
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

        let mut read_record_buffer = [0; 16384];
        let mut write_record_buffer = [0; 16384];
        let config = TlsConfig::<Aes128GcmSha256>::new()
            .with_server_name(dns)
            .enable_rsa_signatures();
        let mut tls = TlsConnection::new(socket, &mut read_record_buffer, &mut write_record_buffer);

        handler.handle_status(MqttStatus::TlsConnect);
        info!("{MODULE} [TLS] Starting handshake");
        tls.open::<_, NoVerify>(TlsContext::new(&config, &mut RoscRng))
            .await?;
        info!("{MODULE} [TLS] Handshake complete");

        let mut config = ClientConfig::new(
            rust_mqtt::client::client_config::MqttVersion::MQTTv5,
            CountingRng(20000),
        );
        // config.add_max_subscribe_qos(QualityOfService::QoS1);
        config.add_client_id(PRODUCT_NAME);
        config.max_packet_size = 100;
        config.add_username(&settings.username);
        config.add_password(&settings.password);
        let mut recv_buffer = [0; 80];
        let mut write_buffer = [0; 80];

        let mut client =
            MqttClient::<_, 5, _>::new(tls, &mut write_buffer, 80, &mut recv_buffer, 80, config);

        handler.handle_status(MqttStatus::MqttConnect);
        info!(
            "{MODULE} Connecting to broker with client_id '{}' and username '{}'",
            PRODUCT_NAME, settings.username
        );
        client.connect_to_broker().await?;
        info!("{MODULE} Connected to broker");

        handler.handle_status(MqttStatus::MqttSubscribe);
        info!("{MODULE} Subscribing to {}", settings.topic);
        client.subscribe_to_topic(&settings.topic).await?;
        info!("{MODULE} Subscribed");

        handler.handle_status(MqttStatus::Connected);
        loop {
            // Use timeout because receive_message and send_ping take &mut self.
            match with_timeout(Duration::from_secs(10), client.receive_message()).await {
                Ok(Ok((topic, body))) => {
                    handler.handle(topic, body);
                }
                Ok(Err(e)) => return Err(e.into()),
                Err(TimeoutError) => {
                    client.send_ping().await?;
                }
            }
        }
    }
    pub fn set_settings(&self, settings: MqttSettings) {
        info!("{MODULE} Updating mqtt settings");
        self.signal.signal(settings);
    }
}

impl Display for MqttStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            MqttStatus::Disconnected => write!(f, "Disconnected"),
            MqttStatus::Connected => write!(f, "Connected"),
            MqttStatus::WaitingForLink => write!(f, "Waiting for link"),
            MqttStatus::WaitingForDhcp => write!(f, "Waiting for DHCP"),
            MqttStatus::DnsQuery => write!(f, "Resolving hostname"),
            MqttStatus::TcpConnect => write!(f, "Establishing TCP connection"),
            MqttStatus::TlsConnect => write!(f, "Establishing TLS connection"),
            MqttStatus::MqttConnect => write!(f, "Establishing MQTT connection"),
            MqttStatus::MqttSubscribe => write!(f, "Subscribing to topic"),
            MqttStatus::Error(e) => write!(f, "{}", e),
        }
    }
}

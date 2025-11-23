use crate::error::Error;
use crate::wifi::WifiModule;
use core::fmt::{write, Display, Formatter};
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_futures::yield_now;
use embassy_net::dns::DnsQueryType;
use embassy_net::tcp::TcpSocket;
use embassy_rp::clocks::RoscRng;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{with_timeout, Duration, TimeoutError, Timer};
use embedded_tls::{Aes128GcmSha256, NoVerify, TlsConfig, TlsConnection, TlsContext};
use log::{error, info};
use rust_mqtt::client::client::MqttClient;
use rust_mqtt::client::client_config::ClientConfig;
use rust_mqtt::packet::v5::reason_codes::ReasonCode;
use rust_mqtt::utils::rng_generator::CountingRng;
use serde::{Deserialize, Serialize};
use static_cell::StaticCell;
use trouble_host::prelude::HeaplessString;

pub struct MqttModuleBuilder {
    pub spawner: Spawner,
    pub stack: &'static embassy_net::Stack<'static>,
}

pub struct MqttModule {
    stack: &'static embassy_net::Stack<'static>,
    signal: Signal<NoopRawMutex, MqttSettings>,
}

pub struct MqttTask {
    module: &'static MqttModule,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct MqttSettings {
    pub hostname: HeaplessString<128>,
    pub port: u16,
    pub username: HeaplessString<128>,
    pub password: HeaplessString<128>,
    pub topic: HeaplessString<128>,
}

impl MqttTask {
    pub fn spawn(self, spawner: Spawner, handler: &'static dyn MqttHandler) -> Result<(), Error> {
        spawner.spawn(mqtt_task(self.module, handler)?);
        Ok(())
    }
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

#[embassy_executor::task]
async fn mqtt_task(module: &'static MqttModule, handler: &'static dyn MqttHandler) {
    let mut settings = module.signal.wait().await;
    loop {
        // Cancel the MQTT connection every time the settings change.
        match select(
            mqtt_runner(module, &settings, handler),
            module.signal.wait(),
        )
        .await
        {
            Either::First(Ok(x)) => match x {},
            Either::First(Err(e)) => {
                error!("[MQTT] error: {:?}", e);
                handler.handle_status(MqttStatus::Error(e));
                Timer::after(Duration::from_secs(10)).await;
            }
            Either::Second(s) => {
                info!("Updating MQTT settings");
                settings = s
            }
        }
    }
}

async fn mqtt_runner(
    module: &'static MqttModule,
    settings: &MqttSettings,
    handler: &'static dyn MqttHandler,
) -> Result<!, Error> {
    handler.handle_status(MqttStatus::WaitingForLink);
    module.stack.wait_link_up().await;
    handler.handle_status(MqttStatus::WaitingForDhcp);
    module.stack.wait_config_up().await;
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut socket = TcpSocket::new(*module.stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(20)));
    let dns = &*settings.hostname;
    let port = settings.port;
    info!("[MQTT] Looking up DNS {:?}", dns);
    handler.handle_status(MqttStatus::DnsQuery);
    let address = module.stack.dns_query(dns, DnsQueryType::A).await?[0];

    let remote_endpoint = (address, port);
    info!("[MQTT] Connecting to address {:?}", remote_endpoint);
    handler.handle_status(MqttStatus::TcpConnect);
    socket.connect(remote_endpoint).await?;
    info!("[MQTT] Connected to TCP");

    let mut read_record_buffer = [0; 16384];
    let mut write_record_buffer = [0; 16384];
    let config = TlsConfig::<Aes128GcmSha256>::new()
        .with_server_name(dns)
        .enable_rsa_signatures();
    let mut tls = TlsConnection::new(socket, &mut read_record_buffer, &mut write_record_buffer);

    handler.handle_status(MqttStatus::TlsConnect);
    tls.open::<_, NoVerify>(TlsContext::new(&config, &mut RoscRng))
        .await?;
    info!("[MQTT] Connected to TLS");

    let mut config = ClientConfig::new(
        rust_mqtt::client::client_config::MqttVersion::MQTTv5,
        CountingRng(20000),
    );
    // config.add_max_subscribe_qos(QualityOfService::QoS1);
    config.add_client_id("flappy");
    config.max_packet_size = 100;
    config.add_username(&settings.username);
    config.add_password(&settings.password);
    let mut recv_buffer = [0; 80];
    let mut write_buffer = [0; 80];

    let mut client =
        MqttClient::<_, 5, _>::new(tls, &mut write_buffer, 80, &mut recv_buffer, 80, config);

    handler.handle_status(MqttStatus::MqttConnect);
    client.connect_to_broker().await?;
    info!("[MQTT] Connected to MQTT Server");

    handler.handle_status(MqttStatus::MqttSubscribe);
    client.subscribe_to_topic(&settings.topic).await?;

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

impl MqttModuleBuilder {
    pub async fn build(self) -> Result<(MqttTask, &'static MqttModule), Error> {
        static MODULE: StaticCell<MqttModule> = StaticCell::new();
        let module = MODULE.init(MqttModule {
            stack: self.stack,
            signal: Signal::new(),
        });

        Ok((MqttTask { module }, module))
    }
}

impl MqttModule {
    pub fn set_settings(&self, settings: MqttSettings) {
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

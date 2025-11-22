use crate::error::Error;
use crate::secrets::{MQTT_PASSWORD, MQTT_USERNAME};
use crate::wifi::WifiModule;
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_net::dns::DnsQueryType;
use embassy_net::tcp::TcpSocket;
use embassy_rp::clocks::RoscRng;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use embedded_tls::{Aes128GcmSha256, NoVerify, TlsConfig, TlsConnection, TlsContext};
use log::info;
use rust_mqtt::client::client::MqttClient;
use rust_mqtt::client::client_config::ClientConfig;
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

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct MqttSettings {
    pub hostname: HeaplessString<256>,
    pub port: u16,
    pub username: HeaplessString<128>,
    pub password: HeaplessString<128>,
    pub topic: HeaplessString<128>,
}

#[embassy_executor::task]
async fn mqtt_task(module: &'static MqttModule) {}

async fn mqtt_runner(module: &'static MqttModule, settings: MqttSettings) -> Result<(), Error> {
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut socket = TcpSocket::new(*module.stack, &mut rx_buffer, &mut tx_buffer);
    // socket.set_timeout(Some(Duration::from_secs(10)));
    let dns = &*settings.hostname;
    let port = settings.port;
    info!("[MQTT] Looking up DNS {:?}", dns);
    let address = module.stack.dns_query(dns, DnsQueryType::A).await?[0];

    let remote_endpoint = (address, port);
    info!("[MQTT] Connecting to address {:?}", remote_endpoint);
    socket.connect(remote_endpoint).await?;
    info!("[MQTT] Connected to TCP");

    let mut read_record_buffer = [0; 16384];
    let mut write_record_buffer = [0; 16384];
    let config = TlsConfig::<Aes128GcmSha256>::new()
        .with_server_name(dns)
        .enable_rsa_signatures();
    let mut tls = TlsConnection::new(socket, &mut read_record_buffer, &mut write_record_buffer);

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

    client.connect_to_broker().await?;
    info!("[MQTT] Connected to MQTT Server");

    client.subscribe_to_topic(&settings.topic).await?;

    loop {
        let (a, b) = client.receive_message().await?;
        info!("[MQTT] a={}", a);
        yield_now().await;
        info!("[MQTT] b={:?}", b);
    }

    Timer::after(Duration::from_secs(10000)).await;
}

impl MqttModuleBuilder {
    pub async fn build(self) -> Result<&'static MqttModule, Error> {
        static MODULE: StaticCell<MqttModule> = StaticCell::new();
        let module = MODULE.init(MqttModule {
            stack: self.stack,
            signal: Signal::new(),
        });
        self.spawner.spawn(mqtt_task(module)?);
        Ok(module)
    }
}

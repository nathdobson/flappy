#![allow(unreachable_code)]
#![allow(unused_imports)]

use arena::Arena;
use core::fmt::{Debug, Display, Formatter};
use core::time::Duration;
use embassy_futures::select::{Either4, select4, select3, Either3};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embedded_io_async::{BufRead, ErrorKind, ErrorType, Read, Write};
use io_adapters::split::split_io;
use io_adapters::tokio::{TokioErrorAdapter, TokioStreamAdapter};
use mqtt_client::Error;
use mqtt_client::client::{ConnectRequest, MqttClient, PublishRequest};
use mqtt_core::protocol::Qos;
use std::env;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWriteExt, ReadBuf};
use tokio::io::{AsyncReadExt, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::{TlsConnector, rustls};

const KEEPALIVE: u16 = 60;

type MyError = Error<TokioErrorAdapter, TokioErrorAdapter>;

#[ignore]
#[tokio::test]
async fn test() -> Result<(), MyError> {
    let host = env::var("MQTT_HOST").unwrap();
    let port = env::var("MQTT_PORT").unwrap().parse::<u16>().unwrap();
    let username = env::var("MQTT_USERNAME").unwrap();
    let password = env::var("MQTT_PASSWORD").unwrap();
    let mut root_cert_store = rustls::RootCertStore::empty();
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));
    let stream = TcpStream::connect((host.clone(), port)).await.unwrap();
    let stream = connector
        .connect(ServerName::try_from(host).unwrap(), stream)
        .await
        .unwrap();
    let (read, write) = split_io(stream);
    let client = MqttClient::<
        NoopRawMutex,
        TokioStreamAdapter<_>,
        TokioStreamAdapter<_>,
        1024,
        1,
        1,
    >::new(TokioStreamAdapter(write), TokioStreamAdapter(read));
    match select3(
        async {
            println!("Connecting...");
            client
                .connect(&ConnectRequest {
                    client_id: "sfasfgasfgf",
                    username: Some(&username),
                    password: Some(&password),
                    keepalive: KEEPALIVE,
                })
                .await?;
            println!("Subscribing...");
            client.subscribe("testtopic/test").await?;
            loop {
                client
                    .publish(&PublishRequest {
                        retain: false,
                        qos: Qos::AtMostOnce,
                        topic: "testtopic/test",
                        payload: b"!!!!",
                    })
                    .await?;
                tokio::time::sleep(Duration::from_secs(10)).await;
            }

            Result::<_, MyError>::Ok(())
        },
        async {
            let mut arena = [0u8; 1024];
            loop {
                let arena = Arena::new(&mut arena)?;
                let (token, packet) = client.receive(arena).await?;
                println!("Received {:?}", packet);
                client.acknowledge(token)?;
            }
            Result::<_, MyError>::Ok(())
        },
        async {
            loop {
                tokio::time::sleep(Duration::from_secs(KEEPALIVE as u64)).await;
                client.ping().await?;
            }
            Result::<_, MyError>::Ok(())
        },
    )
    .await
    {
        Either3::First(x) => x?,
        Either3::Second(x) => x?,
        Either3::Third(x) => x?,
    }
    Ok(())
}

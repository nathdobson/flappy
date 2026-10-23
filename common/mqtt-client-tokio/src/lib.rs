#![allow(unused_imports)]

use arena::Arena;
use core::fmt::{Debug, Display, Formatter};
use embassy_futures::select::{Either3, Either4, select3, select4};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embedded_io_async::{BufRead, ErrorKind, ErrorType, Read, Write};
use io_adapters::split::split_io;
use io_adapters::tokio::{TokioErrorAdapter, TokioStreamAdapter};
use mqtt_client::client::{ConnectRequest, MqttClient, PublishRequest};
use mqtt_core::protocol::{ Qos};
use std::env;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;
use thiserror::Error;
use tokio::io;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWriteExt, ReadBuf};
use tokio::io::{AsyncReadExt, AsyncWrite};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::{TlsConnector, rustls};

pub struct MqttBuilder {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub keepalive: u16,
    pub client_id: String,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("mqtt error")]
    MqttClientError(#[from] mqtt_client::Error<TokioErrorAdapter, TokioErrorAdapter>),
    #[error("alloc error")]
    AllocError(#[from] std::alloc::AllocError),
}

pub struct TokioMqttPublishRequest {
    pub qos: Qos,
    pub topic: String,
    pub payload: Vec<u8>,
    pub retain: bool,
}

impl MqttBuilder {
    pub async fn run(
        self,
        mut requests: mpsc::Receiver<TokioMqttPublishRequest>,
    ) -> Result<(), Error> {
        let mut root_cert_store = rustls::RootCertStore::empty();
        root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(config));
        let stream = TcpStream::connect((self.host.clone(), self.port))
            .await
            .unwrap();
        let stream = connector
            .connect(ServerName::try_from(self.host).unwrap(), stream)
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
                        client_id: &self.client_id,
                        username: Some(&self.username),
                        password: Some(&self.password),
                        keepalive: self.keepalive,
                    })
                    .await?;
                println!("Subscribing...");
                client.subscribe("testtopic/test").await?;
                while let Some(request) = requests.recv().await {
                    client
                        .publish(&PublishRequest {
                            qos: request.qos,
                            topic: &request.topic,
                            payload: &request.payload,
                            retain: request.retain,
                        })
                        .await?;
                }

                Result::<_, Error>::Ok(())
            },
            async {
                let mut arena = [0u8; 1024];
                loop {
                    let arena = Arena::new(&mut arena)?;
                    let (token, packet) = client.receive(arena).await?;
                    println!("Received {:?}", packet);
                    client.acknowledge(token)?;
                }
                #[allow(unreachable_code)]
                Result::<_, Error>::Ok(())
            },
            async {
                loop {
                    tokio::time::sleep(Duration::from_secs(self.keepalive as u64)).await;
                    client.ping().await?;
                }
                #[allow(unreachable_code)]
                Result::<_, Error>::Ok(())
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
}

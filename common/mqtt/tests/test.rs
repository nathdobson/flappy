#![allow(unreachable_code)]

use arena::ArenaStorage;
use core::fmt::{Debug, Display, Formatter};
use core::time::Duration;
use embassy_futures::select::{Either4, select4};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embedded_io_async::{BufRead, ErrorKind, ErrorType, Read, Write};
use mqtt::error::Error;
use mqtt::receiver::MqttReceiver;
use mqtt::sender::{ConnectRequest, MqttSender};
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

type MyError = Error<TokioAdapter<io::Error>>;

#[derive(Clone)]
struct TlsAdapter(Arc<Mutex<TlsStream<TcpStream>>>);

impl AsyncWrite for TlsAdapter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut *self.0.lock().unwrap()).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut *self.0.lock().unwrap()).poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut *self.0.lock().unwrap()).poll_shutdown(cx)
    }
}

impl AsyncRead for TlsAdapter {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *self.0.lock().unwrap()).poll_read(cx, buf)
    }
}

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
    let stream = TlsAdapter(Arc::new(Mutex::new(stream)));
    let sender = MqttSender::<_, 1024, 1, 1>::new(TokioAdapter(stream.clone()));
    let mut receiver = MqttReceiver::new(TokioAdapter(stream.clone()));
    match select4(
        async {
            println!("Connecting...");
            sender
                .connect(&ConnectRequest {
                    client_id: "sfasfgasfgf",
                    username: Some(&username),
                    password: Some(&password),
                })
                .await?;
            println!("Subscribing...");
            sender.subscribe("falafel").await?;
            loop {
                // sender.publish(Qos::AtMostOnce, "falafel", b"HI").await?;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            Result::<_, MyError>::Ok(())
        },
        async {
            let mut arena = ArenaStorage::<1024>::new();
            loop {
                let (token, packet) = receiver.receive(arena.start()).await?;
                println!("Received {:?}", packet);
                sender.acknowledge(token)?;
            }
            Result::<_, MyError>::Ok(())
        },
        async {
            sender.send_acks().await?;
            Result::<_, MyError>::Ok(())
        },
        async {
            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;
                sender.ping().await?;
            }
            Result::<_, MyError>::Ok(())
        },
    )
    .await
    {
        Either4::First(x) => x?,
        Either4::Second(x) => x?,
        Either4::Third(x) => x?,
        Either4::Fourth(x) => x?,
    }
    Ok(())
}

#[derive(Debug)]
pub struct TokioAdapter<T>(T);

impl<T> ErrorType for TokioAdapter<T> {
    type Error = TokioAdapter<io::Error>;
}

impl core::error::Error for TokioAdapter<io::Error> {}

impl Display for TokioAdapter<io::Error> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl embedded_io_async::Error for TokioAdapter<io::Error> {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

impl<T: Unpin + AsyncWrite> Write for TokioAdapter<T> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.0.write(buf).await.map_err(TokioAdapter)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush().await.map_err(TokioAdapter)
    }
}

impl<T: Unpin + AsyncRead> Read for TokioAdapter<T> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.0.read(buf).await.map_err(TokioAdapter)
    }
}

impl<T: Unpin + AsyncBufRead> BufRead for TokioAdapter<T> {
    async fn fill_buf(&mut self) -> Result<&[u8], Self::Error> {
        self.0.fill_buf().await.map_err(TokioAdapter)
    }

    fn consume(&mut self, amt: usize) {
        self.0.consume(amt)
    }
}

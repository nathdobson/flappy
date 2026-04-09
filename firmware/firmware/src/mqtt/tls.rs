use super::MODULE;
use crate::mqtt::merge_socket::MergeSocket;
use crate::mqtt::webpki_provider::WebPkiProvider;
use core::fmt;
use embassy_net::Stack;
use embassy_net::tcp::{TcpReader, TcpSocket, TcpWriter};
use embassy_rp::clocks::RoscRng;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::watch::Watch;
use embassy_time::Duration;
use embedded_tls::{Aes128GcmSha256, Certificate, TlsConfig, TlsConnection, TlsContext};
use log::info;
use protocol::error::MqttServiceError;
use protocol::setup::MqttServiceStatus;
use smoltcp::wire::{DnsQueryType, IpAddress, IpEndpoint};
use crate::mqtt::error::{convert_dns_error, convert_tcp_error, convert_tls_error};

pub struct TlsConnectionBuilder<'a> {
    pub rx_buffer: &'a mut [u8],
    pub tx_buffer: &'a mut [u8],
    pub read_record_buffer: &'a mut [u8],
    pub write_record_buffer: &'a mut [u8],
    pub hostname: &'a str,
    pub port: u16,
    pub stack: &'a Stack<'a>,
}

pub struct TlsConnectionBuilderWithDns<'a> {
    rx_buffer: &'a mut [u8],
    tx_buffer: &'a mut [u8],
    read_record_buffer: &'a mut [u8],
    write_record_buffer: &'a mut [u8],
    hostname: &'a str,
    port: u16,
    stack: &'a Stack<'a>,
    remote_endpoint: IpEndpoint,
}

pub struct TlsConnectionBuilderWithTcp<'a> {
    read_record_buffer: &'a mut [u8],
    write_record_buffer: &'a mut [u8],
    hostname: &'a str,
    stack: &'a Stack<'a>,
    socket: TcpSocket<'a>,
}

pub struct TlsConnectionBuilderWithMergeSocket<'a> {
    read_record_buffer: &'a mut [u8],
    write_record_buffer: &'a mut [u8],
    hostname: &'a str,
    stack: &'a Stack<'a>,
    merge_socket: MergeSocket<TcpWriter<'a>, TcpReader<'a>>,
}

impl<'a> TlsConnectionBuilder<'a> {
    pub async fn resolve_dns<'b>(
        &'b mut self,
    ) -> Result<TlsConnectionBuilderWithDns<'b>, MqttServiceError> {
        info!("{MODULE} [DNS] Querying {:?}", self.hostname);
        let address = self
            .stack
            .dns_query(self.hostname, DnsQueryType::A)
            .await
            .map_err(convert_dns_error)?[0];
        let remote_endpoint = IpEndpoint {
            addr: address,
            port: self.port,
        };
        info!("{MODULE} [DNS] Resolved {}", remote_endpoint);
        Ok(TlsConnectionBuilderWithDns {
            rx_buffer: self.rx_buffer,
            tx_buffer: self.tx_buffer,
            read_record_buffer: self.read_record_buffer,
            write_record_buffer: self.write_record_buffer,
            hostname: self.hostname,
            port: self.port,
            stack: self.stack,
            remote_endpoint,
        })
    }
}

impl<'a> TlsConnectionBuilderWithDns<'a> {
    pub async fn connect_tcp<'b>(
        &'b mut self,
    ) -> Result<TlsConnectionBuilderWithTcp<'b>, MqttServiceError> {
        info!("{MODULE} [TCP] Connecting to {}", self.remote_endpoint);
        let mut socket = TcpSocket::new(*self.stack, self.rx_buffer, self.tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(60)));

        socket
            .connect(self.remote_endpoint)
            .await
            .map_err(convert_tcp_error)?;
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

        Ok(TlsConnectionBuilderWithTcp {
            read_record_buffer: self.read_record_buffer,
            write_record_buffer: self.write_record_buffer,
            hostname: self.hostname,
            stack: self.stack,
            socket,
        })
    }
}

impl<'a> TlsConnectionBuilderWithTcp<'a> {
    pub fn merge_socket<'b>(&'b mut self) -> TlsConnectionBuilderWithMergeSocket<'b> {
        let (read, write) = self.socket.split();
        let merge_socket = MergeSocket::new(write, read);
        TlsConnectionBuilderWithMergeSocket {
            read_record_buffer: self.read_record_buffer,
            write_record_buffer: self.write_record_buffer,
            hostname: self.hostname,
            stack: self.stack,
            merge_socket,
        }
    }
}

impl<'a> TlsConnectionBuilderWithMergeSocket<'a> {
    pub async fn connect_tls<'b>(
        &'b mut self,
    ) -> Result<
        TlsConnection<'b, &'b MergeSocket<TcpWriter<'a>, TcpReader<'a>>, Aes128GcmSha256>,
        MqttServiceError,
    > {
        info!("{MODULE} [TLS] Starting handshake");
        let config = TlsConfig::new()
            .with_server_name(self.hostname)
            .enable_rsa_signatures()
            .with_ca(Certificate::X509(
                mozilla_root_ca::pem::PEM_BUNDLE.as_bytes(),
            ));
        let mut tls = TlsConnection::<_, Aes128GcmSha256>::new(
            &self.merge_socket,
            &mut self.read_record_buffer,
            &mut self.write_record_buffer,
        );

        tls.open::<_>(TlsContext::new(&config, WebPkiProvider::new(RoscRng)))
            .await
            .map_err(convert_tls_error)?;
        info!("{MODULE} [TLS] Handshake complete");
        Ok(tls)
    }
}

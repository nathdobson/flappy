#![no_std]
mod merge_socket;
mod webpki_provider;
// mod fixed_provider;

use crate::merge_socket::MergeSocket;
use crate::webpki_provider::WebPkiProvider;
use core::fmt;
use embassy_net::Stack;
use embassy_net::dns::DnsQueryType;
use embassy_net::tcp::{TcpReader, TcpSocket, TcpWriter};
use embassy_net::wire::IpEndpoint;
use embassy_rp::clocks::RoscRng;
use embassy_time::Duration;
use embedded_tls::{
    Aes256GcmSha384, TlsConfig, TlsConnection, TlsContext, TlsError, TlsReader, TlsWriter,
};
use log::info;

pub struct TlsConnectionBuilder<'a> {
    pub rx_buffer: &'a mut [u8],
    pub tx_buffer: &'a mut [u8],
    pub read_record_buffer: &'a mut [u8],
    pub write_record_buffer: &'a mut [u8],
    pub hostname: &'a str,
    pub port: u16,
    pub stack: Stack<'static>,
}

pub struct TlsConnectionBuilderWithDns<'a> {
    rx_buffer: &'a mut [u8],
    tx_buffer: &'a mut [u8],
    read_record_buffer: &'a mut [u8],
    write_record_buffer: &'a mut [u8],
    hostname: &'a str,
    stack: Stack<'static>,
    remote_endpoint: IpEndpoint,
}

pub struct TlsConnectionBuilderWithTcp<'a> {
    read_record_buffer: &'a mut [u8],
    write_record_buffer: &'a mut [u8],
    hostname: &'a str,
    socket: TcpSocket<'a, 'static>,
    remote_endpoint: IpEndpoint,
}

pub struct TlsConnectionBuilderWithTcpConnected<'a> {
    read_record_buffer: &'a mut [u8],
    write_record_buffer: &'a mut [u8],
    hostname: &'a str,
    socket: TcpSocket<'a, 'static>,
}

pub struct TlsConnectionBuilderWithMergeSocket<'a> {
    read_record_buffer: &'a mut [u8],
    write_record_buffer: &'a mut [u8],
    hostname: &'a str,
    merge_socket: MergeSocket<TcpWriter<'a, 'static>, TcpReader<'a, 'static>>,
}

impl<'a> TlsConnectionBuilder<'a> {
    pub async fn resolve_dns<'b>(
        &'b mut self,
    ) -> Result<TlsConnectionBuilderWithDns<'b>, embassy_net::dns::Error> {
        info!("[DNS] Querying {:?}", self.hostname);
        let address = self.stack.dns_query(self.hostname, DnsQueryType::A).await?[0];
        let remote_endpoint = IpEndpoint {
            addr: address,
            port: self.port,
        };
        info!("[DNS] Resolved {}", remote_endpoint);
        Ok(TlsConnectionBuilderWithDns {
            rx_buffer: self.rx_buffer,
            tx_buffer: self.tx_buffer,
            read_record_buffer: self.read_record_buffer,
            write_record_buffer: self.write_record_buffer,
            hostname: self.hostname,
            stack: self.stack,
            remote_endpoint,
        })
    }
}

impl<'a> TlsConnectionBuilderWithDns<'a> {
    pub async fn connect_tcp<'b>(
        &'b mut self,
    ) -> Result<TlsConnectionBuilderWithTcp<'b>, embassy_net::Full> {
        let socket = TcpSocket::new(self.stack, self.rx_buffer, self.tx_buffer)?;
        Ok(TlsConnectionBuilderWithTcp {
            read_record_buffer: self.read_record_buffer,
            write_record_buffer: self.write_record_buffer,
            hostname: self.hostname,
            socket,
            remote_endpoint: self.remote_endpoint,
        })
    }
}

impl<'a> TlsConnectionBuilderWithTcp<'a> {
    pub async fn connect_tcp(
        mut self,
    ) -> Result<TlsConnectionBuilderWithTcpConnected<'a>, embassy_net::tcp::ConnectError> {
        info!("[TCP] Connecting to {}", self.remote_endpoint);
        self.socket.set_timeout(Some(Duration::from_secs(60)));

        self.socket.connect(self.remote_endpoint).await?;
        info!(
            "[TCP] Connected ({} -> {})",
            fmt::from_fn(|f| {
                if let Some(local) = self.socket.local_endpoint() {
                    write!(f, "{}", local)?
                }
                Ok(())
            }),
            fmt::from_fn(|f| {
                if let Some(remote) = self.socket.remote_endpoint() {
                    write!(f, "{}", remote)?
                }
                Ok(())
            }),
        );

        Ok(TlsConnectionBuilderWithTcpConnected {
            read_record_buffer: self.read_record_buffer,
            write_record_buffer: self.write_record_buffer,
            hostname: self.hostname,
            socket: self.socket,
        })
    }
}

impl<'a> TlsConnectionBuilderWithTcpConnected<'a> {
    pub fn merge_socket<'b>(&'b mut self) -> TlsConnectionBuilderWithMergeSocket<'b> {
        let (read, write) = self.socket.split();
        let merge_socket = MergeSocket::new(write, read);
        TlsConnectionBuilderWithMergeSocket {
            read_record_buffer: self.read_record_buffer,
            write_record_buffer: self.write_record_buffer,
            hostname: self.hostname,
            merge_socket,
        }
    }
}

// type FlappyCipherSuite = Aes128GcmSha256;
type FlappyCipherSuite = Aes256GcmSha384;

pub type FlappyTlsConnection<'b, 'a> = TlsConnection<
    'b,
    &'b MergeSocket<TcpWriter<'a, 'static>, TcpReader<'a, 'static>>,
    FlappyCipherSuite,
>;

pub type FlappyTlsWriter<'a> = TlsWriter<
    'a,
    &'a MergeSocket<TcpWriter<'a, 'static>, TcpReader<'a, 'static>>,
    FlappyCipherSuite,
>;

pub type FlappyTlsReader<'a> = TlsReader<
    'a,
    &'a MergeSocket<TcpWriter<'a, 'static>, TcpReader<'a, 'static>>,
    FlappyCipherSuite,
>;

impl<'a> TlsConnectionBuilderWithMergeSocket<'a> {
    pub async fn connect_tls<'b>(&'b mut self) -> Result<FlappyTlsConnection<'b, 'a>, TlsError> {
        info!(" [TLS] Starting handshake");
        let config = TlsConfig::new()
            .with_server_name(self.hostname)
            .enable_rsa_signatures();

        let mut tls = TlsConnection::<_, FlappyCipherSuite>::new(
            &self.merge_socket,
            &mut self.read_record_buffer,
            &mut self.write_record_buffer,
        );
        // let provider = FixedProvider::new(RoscRng);
        // let provider = UnsecureProvider::new(RoscRng).with_cert(Certificate::X509(
        //     include_bytes!("/Users/nathan/Downloads/emqxsl-ca.crt"),
        // ));
        let provider = WebPkiProvider::new(RoscRng);
        tls.open::<_>(TlsContext::new(&config, provider)).await?;
        info!("[TLS] Handshake complete");
        Ok(tls)
    }
}

#![no_std]

mod merge_socket;
// mod webpki_provider;
// mod fixed_provider;

use crate::merge_socket::MergeSocket;
use core::ffi::CStr;
use core::fmt;
use embassy_net::Stack;
use embassy_net::tcp::{TcpReader, TcpSocket, TcpWriter};
use embassy_rp::clocks::RoscRng;
use embassy_time::Duration;
use log::info;
use mbedtls_rs::{
    Certificate, ClientSessionConfig, Session, SessionConfig, SessionError, Tls, TlsError, X509,
};
use smoltcp::wire::{DnsQueryType, IpEndpoint};

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
    stack: &'a Stack<'a>,
    remote_endpoint: IpEndpoint,
}

pub struct TlsConnectionBuilderWithTcp<'a> {
    read_record_buffer: &'a mut [u8],
    write_record_buffer: &'a mut [u8],
    hostname: &'a str,
    socket: TcpSocket<'a>,
}

pub struct TlsConnectionBuilderWithMergeSocket<'a> {
    read_record_buffer: &'a mut [u8],
    write_record_buffer: &'a mut [u8],
    hostname: &'a str,
    merge_socket: MergeSocket<TcpWriter<'a>, TcpReader<'a>>,
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
    ) -> Result<TlsConnectionBuilderWithTcp<'b>, embassy_net::tcp::ConnectError> {
        info!("[TCP] Connecting to {}", self.remote_endpoint);
        let mut socket = TcpSocket::new(*self.stack, self.rx_buffer, self.tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(60)));

        socket.connect(self.remote_endpoint).await?;
        info!(
            "[TCP] Connected ({} -> {})",
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
            merge_socket,
        }
    }
}

const CA_BUNDLE: &CStr = match CStr::from_bytes_with_nul(
    concat!(
        include_str!("../../../submodules/mbedtls-rs/examples/common/certs/ca-bundle-small.pem"),
        "\0"
    )
    .as_bytes(),
) {
    Ok(bundle) => bundle,
    _ => panic!("CA bundle is not a valid text file"),
};

#[derive(Debug)]
pub enum MyError {
    SessionError(SessionError),
    TlsError(TlsError),
}

impl From<TlsError> for MyError {
    fn from(e: TlsError) -> Self {
        MyError::TlsError(e)
    }
}

impl From<SessionError> for MyError {
    fn from(e: SessionError) -> Self {
        MyError::SessionError(e)
    }
}

impl<'a> TlsConnectionBuilderWithMergeSocket<'a> {
    pub async fn connect_tls<'b>(
        &'b mut self,
        tls: &'b Tls<'b>,
    ) -> Result<mbedtls_rs::Session<'b, &'b MergeSocket<TcpWriter<'a>, TcpReader<'a>>>, MyError>
    {
        let mut conf = ClientSessionConfig {
            ca_chain: Some(Certificate::new(X509::PEM(CA_BUNDLE)).unwrap()),
            server_name: Some(c"httpbin.org"),
            ..ClientSessionConfig::new()
        };

        let mut session = Session::new(
            tls.reference(),
            &self.merge_socket,
            &SessionConfig::Client(conf),
        )?;
        Ok(session)
    }
}

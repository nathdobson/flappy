#![allow(unused_imports)]
#![allow(unused_variables)]
#![deny(unused_must_use)]
#![allow(unused_mut)]
#![allow(unreachable_code)]

use rustls::pki_types::InvalidDnsNameError;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::io::{Cursor, ErrorKind};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::try_join;

#[cfg(test)]
mod test;
#[derive(Debug)]
pub enum Error {
    InvalidDnsNameError(InvalidDnsNameError),
    RustlsError(rustls::Error),
    IoError(std::io::Error),
}

impl From<InvalidDnsNameError> for Error {
    fn from(value: InvalidDnsNameError) -> Self {
        Error::InvalidDnsNameError(value)
    }
}
impl From<rustls::Error> for Error {
    fn from(value: rustls::Error) -> Self {
        Error::RustlsError(value)
    }
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IoError(value)
    }
}

pub async fn fetch_certificate_list_sha256(
    domain: String,
    port: u16,
) -> Result<[u8; 32], Error> {
    let root_store =
        rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let rc_config = Arc::new(config);
    let example_com = domain.clone().try_into()?;
    let mut client = rustls::ClientConnection::new(rc_config, example_com)?;
    let mut socket = TcpStream::connect((domain, port)).await?;
    loop {
        {
            let mut send_buf = vec![];
            client.write_tls(&mut send_buf)?;
            socket.write_all(&send_buf).await?;
        }
        {
            let mut receive_buf = [0u8; 1024];
            let len = socket.read(&mut receive_buf).await?;
            let mut cursor = Cursor::new(&receive_buf[..len]);
            client.read_tls(&mut cursor)?;
            assert_eq!(cursor.position() as usize, len);
            client.process_new_packets()?;
        }
        let mut plaintext = Vec::new();
        match client.reader().read_to_end(&mut plaintext) {
            Ok(usize) => {}
            Err(e) if e.kind() == ErrorKind::WouldBlock => {}
            Err(e) => return Err(e.into()),
        }
        if let Some(certs) = client.peer_certificates() {
            let mut sha2 = Sha256::new();
            for cert in certs {
                let cert = &**cert;
                sha2.update(&(cert.len() as u64).to_le_bytes());
                sha2.update(cert);
            }
            return Ok(sha2.finalize().into());
        }
    }
}

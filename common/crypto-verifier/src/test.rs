use crate::FixedProvider;
use embedded_io_adapters::tokio_1::FromTokio;
use embedded_io_async::Write;
use embedded_tls::TlsConfig;
use embedded_tls::TlsConnection;
use embedded_tls::TlsContext;
use embedded_tls::UnsecureProvider;
use embedded_tls::{Aes128GcmSha256, Aes256GcmSha384};
use rand::rngs::OsRng;
use tokio::net::TcpStream;
use hex_literal::hex;

#[tokio::test]
async fn test() {
    simple_logger::init().unwrap();
    let host = "u8c6afc1.ala.us-east-1.emqxsl.com";
    let port = 8883;
    let stream = TcpStream::connect((host, port)).await.unwrap();

    println!("Connected");
    let mut read_record_buffer = [0; 16384];
    let mut write_record_buffer = [0; 16384];
    let config = TlsConfig::new()
        .with_server_name(host)
        .enable_rsa_signatures();
    let mut tls = TlsConnection::<_, Aes256GcmSha384>::new(
        FromTokio::new(stream),
        &mut read_record_buffer,
        &mut write_record_buffer,
    );
    println!("Starting handshake");
    tls.open(TlsContext::new(&config, FixedProvider::new(OsRng, hex!("8075771E5AC95E0810828AB426510790FC01F2087B93AF28C1E9EE21260CDCA1"))))
        .await
        .expect("error establishing TLS connection");
    println!("Ended handshake");
}

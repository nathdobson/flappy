#![no_std]
#![feature(type_alias_impl_trait)]
#![deny(unused_must_use)]
#![allow(unused_features)]
#![allow(unused_imports)]

use core::cell::OnceCell;
use core::net::SocketAddr;
use embassy_net::IpListenEndpoint;
use embassy_net::dns::{DnsQueryType, DnsSocket};
use embassy_net::udp::{BindError, PacketMetadata, UdpSocket};
use embassy_time::Instant;
use log::info;
use make_static::make_static;
use make_static::reexports::static_cell::StaticCell;
use sntpc::{NtpContext, fraction_to_microseconds, get_time};
use sntpc_net_embassy::UdpSocketWrapper;
use sntpc_time_embassy::EmbassyTimestampGenerator;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NtpError {
    #[error("DNS missing IP")]
    DnsMissingIp,
    #[error("DNS error")]
    DnsError(#[from] embassy_net::dns::Error),
    #[error("UDP Bind error")]
    BindError(#[from] BindError),
    #[error("Ntp error")]
    SntpcError(#[from] sntpc::Error),
}

struct NtpBuffers {
    rx_meta: [PacketMetadata; 2],
    tx_meta: [PacketMetadata; 2],
    rx_buffer: [u8; 128],
    tx_buffer: [u8; 128],
}
pub struct NtpClock {
    stack: embassy_net::Stack<'static>,
    buffers: StaticCell<NtpBuffers>,
    offset: OnceCell<i64>,
}

impl NtpClock {
    pub const fn new(stack: embassy_net::Stack<'static>) -> Self {
        NtpClock {
            stack,
            buffers: StaticCell::new(),
            offset: OnceCell::new(),
        }
    }
    pub async fn init(&'static self) -> Result<(), NtpError> {
        let buffers = self.buffers.init_with(|| NtpBuffers {
            rx_meta: [PacketMetadata::EMPTY; 2],
            tx_meta: [PacketMetadata::EMPTY; 2],
            rx_buffer: [0; 128],
            tx_buffer: [0; 128],
        });
        let mut udp = UdpSocket::new(
            self.stack,
            &mut buffers.rx_meta,
            &mut buffers.rx_buffer,
            &mut buffers.tx_meta,
            &mut buffers.tx_buffer,
        );
        let dns = DnsSocket::new(self.stack);
        self.stack.wait_config_up().await;
        let dns = dns
            .query("pool.ntp.org", DnsQueryType::A)
            .await?
            .first()
            .ok_or(NtpError::DnsMissingIp)?
            .clone();
        udp.bind(IpListenEndpoint {
            addr: None,
            port: 123,
        })?;
        let context = NtpContext::new(EmbassyTimestampGenerator::default());
        let time = get_time(
            SocketAddr::new(dns.into(), 123),
            &UdpSocketWrapper::new(udp),
            context,
        )
        .await?;
        self.offset.set(time.offset).unwrap();
        info!("NTP time: {:?} micros", self.now_micros());
        Ok(())
    }
    pub fn now_micros(&self) -> Option<i64> {
        Some(self.offset.get()? + (Instant::now().as_micros() as i64))
    }
}

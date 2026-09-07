#![no_std]
#![feature(type_alias_impl_trait)]
#![deny(unused_must_use)]
#![allow(unused_features)]
#![allow(unused_imports)]

use core::cell::OnceCell;
use core::net::SocketAddr;
use embassy_net::dns::DnsQueryType;
use embassy_net::iface::Iface;
use embassy_net::udp::{BindError, UdpSocket};
use embassy_net::wire::IpListenEndpoint;
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
    #[error("Network buffer full")]
    Full(#[from] embassy_net::Full),
}

pub struct NtpClock {
    stack: embassy_net::Stack<'static>,
    iface: Iface<'static>,
    offset: OnceCell<i64>,
}

impl NtpClock {
    pub const fn new(stack: embassy_net::Stack<'static>, iface: Iface<'static>) -> Self {
        NtpClock {
            stack,
            iface,
            offset: OnceCell::new(),
        }
    }
    pub async fn init(&'static self) -> Result<(), NtpError> {
        let mut udp = UdpSocket::new(self.stack)?;
        self.iface.wait_config_up().await;
        let dns = self
            .stack
            .dns_query("pool.ntp.org", DnsQueryType::A)
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

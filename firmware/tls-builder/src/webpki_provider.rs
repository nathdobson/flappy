use embassy_sync::once_lock::OnceLock;
use embedded_tls::pki::CertVerifier;
use embedded_tls::{
    Certificate, CryptoProvider, CryptoRngCore, TlsCipherSuite, TlsClock, TlsError, TlsVerifier,
};
use heapless::Vec;

pub struct WebPkiProvider<C: TlsCipherSuite, R> {
    rng: R,
    verifier: CertVerifier<'static, C, VeryGoodClock, 4096>,
}

pub struct VeryGoodClock;

impl TlsClock for VeryGoodClock {
    fn now() -> Option<u64> {
        Some(1776976399)
    }
}

static CERTS: OnceLock<Vec<Certificate<&'static [u8]>, 500>> = OnceLock::new();

impl<C: TlsCipherSuite, R> WebPkiProvider<C, R> {
    pub fn new(rng: R) -> Self {
        let certs = CERTS.get_or_init(|| {
            let mut result = Vec::new();
            for cert in mozilla_root_ca::der::DER_LIST {
                result
                    .push(Certificate::X509(*cert))
                    .expect("Too many certs");
            }
            result
        });
        WebPkiProvider {
            rng,
            verifier: CertVerifier::new(certs),
        }
    }
}

impl<C: TlsCipherSuite, R: CryptoRngCore> CryptoProvider for WebPkiProvider<C, R> {
    type CipherSuite = C;
    type Signature = &'static [u8];

    fn rng(&mut self) -> impl CryptoRngCore {
        &mut self.rng
    }
    fn verifier(&mut self) -> Result<&mut impl TlsVerifier<Self::CipherSuite>, TlsError> {
        Ok(&mut self.verifier)
    }
}

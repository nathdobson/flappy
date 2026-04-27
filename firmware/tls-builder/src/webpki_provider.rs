use embassy_sync::once_lock::OnceLock;
use embedded_tls::pki::CertVerifier;
use embedded_tls::{
    Certificate, CryptoProvider, CryptoRngCore, TlsCipherSuite, TlsClock, TlsError, TlsVerifier,
};

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

const CERT_COUNT: usize = 143;
static CERTS: OnceLock<[Certificate<&'static [u8]>; CERT_COUNT]> = OnceLock::new();

impl<C: TlsCipherSuite, R> WebPkiProvider<C, R> {
    pub fn new(rng: R) -> Self {
        let certs = CERTS.get_or_init(|| {
            assert_eq!(CERT_COUNT, mozilla_root_ca::der::DER_LIST.len());
            core::array::from_fn(|n| Certificate::X509(mozilla_root_ca::der::DER_LIST[n]))
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
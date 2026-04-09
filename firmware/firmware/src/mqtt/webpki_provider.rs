use embedded_tls::webpki::CertVerifier;
use embedded_tls::{
    Certificate, CryptoProvider, CryptoRngCore, NoClock, TlsCipherSuite, TlsError, TlsVerifier,
};

pub struct WebPkiProvider<C: TlsCipherSuite, R> {
    rng: R,
    verifier: CertVerifier<C, NoClock, 4096>, //Aes128GcmSha256
}

impl<C: TlsCipherSuite, R> WebPkiProvider<C, R> {
    pub fn new(rng: R) -> Self {
        WebPkiProvider {
            rng,
            verifier: CertVerifier::new(),
        }
    }
}

impl<C: TlsCipherSuite, R: CryptoRngCore> CryptoProvider for WebPkiProvider<C, R> {
    type CipherSuite = C;
    type Signature = p256::ecdsa::DerSignature;

    fn rng(&mut self) -> impl CryptoRngCore {
        &mut self.rng
    }
}

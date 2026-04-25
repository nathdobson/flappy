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

impl<C: TlsCipherSuite, R> WebPkiProvider<C, R> {
    pub fn new(rng: R) -> Self {
        WebPkiProvider {
            rng,
            verifier: CertVerifier::new(Certificate::X509(
                // include_bytes!("../../emqx.der"),
                // include_bytes!("/Users/nathan/Downloads/EncryptionEverywhereDVTLSCA-G2.crt"),
                include_bytes!("/Users/nathan/Downloads/emqxsl-ca.der"),
                // mozilla_root_ca::der::DER_LIST[45],
                // include_bytes!("/Users/nathan/Downloads/test-dv-ecc-ssl-com-chain.der"),
            )),
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

#[unsafe(no_mangle)]
unsafe extern "C" fn ring_core_0_17_16000__sha512_block_data_order_neon() {
    todo!();
}

#[unsafe(no_mangle)]
unsafe extern "C" fn ring_core_0_17_16000__sha512_block_data_order_nohw() {
    todo!();
}

#[unsafe(no_mangle)]
unsafe extern "C" fn __assert_func() {
    todo!();
}

#[unsafe(no_mangle)]
unsafe extern "C" fn ring_core_0_17_16000__sha256_block_data_order_neon() {
    todo!();
}

#[unsafe(no_mangle)]
unsafe extern "C" fn ring_core_0_17_16000__bn_mul8x_mont_neon() {
    todo!();
}

#[unsafe(no_mangle)]
unsafe extern "C" fn ring_core_0_17_16000__sha256_block_data_order_nohw() {
    todo!();
}

#[unsafe(no_mangle)]
unsafe extern "C" fn ring_core_0_17_16000__bn_mul_mont_nohw() {
    todo!();
}

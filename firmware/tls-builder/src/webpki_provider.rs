use embedded_tls::webpki::CertVerifier;
use embedded_tls::{CryptoProvider, CryptoRngCore, NoClock, TlsCipherSuite, TlsError, TlsVerifier};

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
    // fn verifier(&mut self) -> Result<&mut impl TlsVerifier<Self::CipherSuite>, TlsError> {
    //     Ok(&mut self.verifier)
    // }
}

// #[unsafe(no_mangle)]
// unsafe extern "C" fn ring_core_0_17_16000__sha512_block_data_order_neon() {
//     todo!();
// }
//
// #[unsafe(no_mangle)]
// unsafe extern "C" fn ring_core_0_17_16000__sha512_block_data_order_nohw() {
//     todo!();
// }
//
// #[unsafe(no_mangle)]
// unsafe extern "C" fn __assert_func() {
//     todo!();
// }
//
// #[unsafe(no_mangle)]
// unsafe extern "C" fn ring_core_0_17_16000__sha256_block_data_order_neon() {
//     todo!();
// }
//
// #[unsafe(no_mangle)]
// unsafe extern "C" fn ring_core_0_17_16000__bn_mul8x_mont_neon() {
//     todo!();
// }
//
// #[unsafe(no_mangle)]
// unsafe extern "C" fn ring_core_0_17_16000__sha256_block_data_order_nohw() {
//     todo!();
// }
//
// #[unsafe(no_mangle)]
// unsafe extern "C" fn ring_core_0_17_16000__bn_mul_mont_nohw() {
//     todo!();
// }

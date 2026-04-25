use core::marker::PhantomData;
use embedded_tls::{
    CertificateEntryRef, CertificateRef, CertificateVerifyRef, CryptoProvider, CryptoRngCore,
    NoVerify, TlsCipherSuite, TlsError, TlsVerifier,
};
use log::{info, warn};

pub struct FixedProvider<C, R> {
    rng: R,
    verifier: FixedVerifier,
    phantom: PhantomData<C>,
}

impl<C, R> FixedProvider<C, R> {
    pub fn new(rng: R) -> Self {
        FixedProvider {
            rng,
            verifier: FixedVerifier,
            phantom: PhantomData,
        }
    }
}

impl<C: TlsCipherSuite, R: CryptoRngCore> CryptoProvider for FixedProvider<C, R> {
    type CipherSuite = C;
    type Signature = &'static [u8];

    fn rng(&mut self) -> impl CryptoRngCore {
        &mut self.rng
    }
    fn verifier(&mut self) -> Result<&mut impl TlsVerifier<Self::CipherSuite>, crate::TlsError> {
        Ok(&mut self.verifier)
    }
}

pub struct FixedVerifier;

impl<C: TlsCipherSuite> TlsVerifier<C> for FixedVerifier {
    fn set_hostname_verification(&mut self, hostname: &str) -> Result<(), TlsError> {
        Ok(())
    }

    fn verify_certificate(
        &mut self,
        transcript: &C::Hash,
        cert: CertificateRef,
    ) -> Result<(), TlsError> {
        for entry in &cert.entries {
            let bytes = match entry {
                CertificateEntryRef::X509(x509) => x509,
                _ => todo!(),
            };
            if bytes.len() > 50 {
                info!(
                    "Cert: {:?}...{:?}",
                    &bytes[..50],
                    &bytes[bytes.len() - 50..]
                );
            } else {
                info!("Cert: {:?}", bytes);
            }
        }
        if cert.raw_entries.len() > 50 {
            info!(
                    "Cert: {:?}...{:?}",
                    &cert.raw_entries[..50],
                    &cert.raw_entries[cert.raw_entries.len() - 50..]
                );
        } else {
            info!("Cert: {:?}", cert.raw_entries);
        }
        Ok(())
    }

    fn verify_signature(&mut self, verify: CertificateVerifyRef) -> Result<(), TlsError> {
        info!("Verifying signature {:?}", verify);
        Ok(())
    }
}

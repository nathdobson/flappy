#![cfg_attr(not(test), no_std)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![deny(unused_must_use)]
#[cfg(test)]
mod test;

use core::marker::PhantomData;
use embedded_tls::webpki::CertVerifier;
use embedded_tls::{Aes128GcmSha256, Certificate, CertificateEntryRef, CertificateVerifyRef, CryptoProvider, CryptoRng, CryptoRngCore, NoClock, NoSign, NoVerify, SignatureScheme, TlsCipherSuite, TlsClock, TlsError, TlsVerifier};
use log::info;
use p256::SecretKey;
use p256::ecdsa::signature::digest::Digest;
use p256::ecdsa::{SigningKey, signature};
use p256::elliptic_curve::rand_core::RngCore;
use sha2::Sha256;

pub struct FixedProvider<C, R> {
    rng: R,
    phantom: PhantomData<C>,
    certificate_list_sha256: [u8; 32],
}

impl<C, R> FixedProvider<C, R> {
    pub fn new(rng: R, certificate_list_sha256: [u8; 32]) -> Self {
        FixedProvider {
            rng,
            phantom: PhantomData,
            certificate_list_sha256,
        }
    }
}

impl<C: TlsCipherSuite, R> TlsVerifier<C> for FixedProvider<C, R> {
    fn set_hostname_verification(&mut self, hostname: &str) -> Result<(), TlsError> {
        Ok(())
    }

    fn verify_certificate(
        &mut self,
        transcript: &<C as TlsCipherSuite>::Hash,
        //ca: &Option<Certificate>,
        cert: embedded_tls::CertificateRef,
    ) -> Result<(), TlsError> {
        let mut sha2 = Sha256::new();
        for entry in cert.entries {
            match entry {
                CertificateEntryRef::X509(key) => {
                    sha2.update((key.len() as u64).to_le_bytes().as_ref());
                    sha2.update(key);
                }
                CertificateEntryRef::RawPublicKey(key) => {
                    return Err(TlsError::InvalidCertificate);
                }
            }
        }
        let sha2 = sha2.finalize();
        if sha2 == self.certificate_list_sha256.into() {
            Ok(())
        } else {
            Err(TlsError::InvalidCertificate)
        }
    }

    fn verify_signature(&mut self, verify: CertificateVerifyRef) -> Result<(), TlsError> {
        Ok(())
    }
}

pub struct WebPkiProvider<'a, C: TlsCipherSuite, R> {
    pub rng: R,
    pub verifier: CertVerifier<'a, C, NoClock, 4096>, //Aes128GcmSha256
}

impl<C: TlsCipherSuite, R: CryptoRngCore> CryptoProvider for FixedProvider<C, R> {
    type CipherSuite = C;
    type Signature = p256::ecdsa::DerSignature;

    fn rng(&mut self) -> impl CryptoRngCore {
        &mut self.rng
    }

    fn verifier(&mut self) -> Result<&mut impl TlsVerifier<Self::CipherSuite>, TlsError> {
        Ok(self)
    }
}

impl<'a, C: TlsCipherSuite, R: CryptoRngCore> CryptoProvider for WebPkiProvider<'a, C, R> {
    type CipherSuite = C;
    type Signature = p256::ecdsa::DerSignature;

    fn rng(&mut self) -> impl CryptoRngCore {
        &mut self.rng
    }
}

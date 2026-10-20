//! Initial trust confirmation and certificate-authority pinning for NAS connections.
//! The initial connection captures the NAS authority and displays its fingerprint.
//! Subsequent connections trust only the confirmed authority regardless of address changes.

use crate::error::ClientError;
use rustls::{
    client::{
        danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
        WebPkiServerVerifier,
    },
    crypto::{ring::default_provider, CryptoProvider},
    pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer, ServerName, UnixTime},
    CertificateError, ClientConfig, DigitallySignedStruct, Error, RootCertStore, SignatureScheme,
};
use std::sync::{Arc, Mutex};

fn provider() -> Arc<CryptoProvider> {
    Arc::new(default_provider())
}

/// The SHA-256 of some bytes, in lower-case hexadecimal.
pub fn sha256_hex(bytes: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The certificates of a PEM text.
pub fn certificates(pem: &str) -> Vec<CertificateDer<'static>> {
    CertificateDer::pem_slice_iter(pem.as_bytes())
        .filter_map(Result::ok)
        .collect()
}

/// The fingerprint of the authority in a PEM text, when there is one.
pub fn authority_fingerprint(pem: &str) -> Option<String> {
    certificates(pem).first().map(|der| sha256_hex(der))
}

fn name_only(error: &Error) -> bool {
    matches!(
        error,
        Error::InvalidCertificate(
            CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. }
        )
    )
}

fn verifier(authority_pem: &str) -> Result<Arc<WebPkiServerVerifier>, ClientError> {
    let mut roots = RootCertStore::empty();
    for certificate in certificates(authority_pem) {
        roots.add(certificate).map_err(|_| ClientError::NotANas)?;
    }
    if roots.is_empty() {
        return Err(ClientError::NotANas);
    }
    WebPkiServerVerifier::builder_with_provider(Arc::new(roots), provider())
        .build()
        .map_err(|_| ClientError::NotANas)
}

/// Verifies the NAS certificate authority independently of the connection hostname.
/// NAS certificates may not contain every address assigned by the local network.
#[derive(Debug)]
struct Pinned(Arc<WebPkiServerVerifier>);

impl ServerCertVerifier for Pinned {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        match self
            .0
            .verify_server_cert(end_entity, intermediates, server_name, ocsp_response, now)
        {
            Err(error) if name_only(&error) => Ok(ServerCertVerified::assertion()),
            other => other,
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.0
            .verify_tls12_signature(message, certificate, signature)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.0
            .verify_tls13_signature(message, certificate, signature)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.supported_verify_schemes()
    }
}

/// Captures the initial server certificate chain before authority fingerprint confirmation.
#[derive(Debug, Default)]
pub struct Capture(Mutex<Vec<CertificateDer<'static>>>);

impl Capture {
    /// The certificate the server showed and the ones it sent with it.
    pub fn chain(&self) -> Vec<CertificateDer<'static>> {
        self.0.lock().map(|chain| chain.clone()).unwrap_or_default()
    }
}

impl ServerCertVerifier for Capture {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        if let Ok(mut chain) = self.0.lock() {
            *chain = std::iter::once(end_entity)
                .chain(intermediates)
                .map(|certificate| certificate.clone().into_owned())
                .collect();
        }
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            certificate,
            signature,
            &provider().signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            certificate,
            signature,
            &provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn builder() -> Result<rustls::ConfigBuilder<ClientConfig, rustls::WantsVerifier>, ClientError> {
    ClientConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .map_err(|_| ClientError::NetworkUnavailable)
}

/// Initial connection settings using certificate capture before trust confirmation.
pub fn capturing() -> Result<(ClientConfig, Arc<Capture>), ClientError> {
    let capture = Arc::new(Capture::default());
    let config = builder()?
        .dangerous()
        .with_custom_certificate_verifier(capture.clone())
        .with_no_client_auth();
    Ok((config, capture))
}

/// Whether the certificate a server showed comes from the authority, whatever names it holds.
pub fn issued_by(authority_pem: &str, chain: &[CertificateDer<'static>]) -> bool {
    let (Ok(verifier), Some((end_entity, intermediates))) =
        (verifier(authority_pem), chain.split_first())
    else {
        return false;
    };
    let Ok(name) = ServerName::try_from("nas.invalid") else {
        return false;
    };
    match verifier.verify_server_cert(end_entity, intermediates, &name, &[], UnixTime::now()) {
        Ok(_) => true,
        Err(error) => name_only(&error),
    }
}

/// Connection settings restricted to the trusted NAS authority and optional device certificate.
pub fn pinned(
    authority_pem: &str,
    identity: Option<(&str, &str)>,
) -> Result<ClientConfig, ClientError> {
    let pinned = builder()?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(Pinned(verifier(authority_pem)?)));
    match identity {
        None => Ok(pinned.with_no_client_auth()),
        Some((certificate, key)) => {
            let key = PrivateKeyDer::from_pem_slice(key.as_bytes())
                .map_err(|_| ClientError::SecretUnavailable)?;
            pinned
                .with_client_auth_cert(certificates(certificate), key)
                .map_err(|_| ClientError::SecretUnavailable)
        }
    }
}

#[cfg(test)]
#[path = "../tests/tls/mod.rs"]
mod tests;

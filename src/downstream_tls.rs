//! Transport-neutral downstream TLS configuration owned by the edge ingress boundary.
//!
//! This module describes only what the gateway needs to consume an already-provisioned server
//! certificate and negotiate an admitted downstream application protocol. Certificate issuance,
//! renewal, backup, identity lifecycle and private-key custody remain outside the gateway.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// Downstream application-protocol policy admitted by the current TLS contract.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownstreamAlpnPolicy {
    /// Prefer HTTP/2 while explicitly retaining HTTP/1.1 fallback through ALPN.
    H2Http1,
}

/// References to server-side TLS material supplied by the canonical certificate/secret owner.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DownstreamTlsConfig {
    certificate_chain_file: PathBuf,
    private_key_file: PathBuf,
    alpn: DownstreamAlpnPolicy,
}

/// Reasons a downstream TLS declaration cannot become listener authority.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DownstreamTlsConfigError {
    /// The certificate-chain reference is empty or whitespace-only.
    #[error("downstream TLS certificate_chain_file must not be empty")]
    EmptyCertificateChainFile,
    /// The certificate-chain reference is relative and could change meaning with process cwd.
    #[error("downstream TLS certificate_chain_file must be an absolute path")]
    RelativeCertificateChainFile,
    /// The private-key reference is empty or whitespace-only.
    #[error("downstream TLS private_key_file must not be empty")]
    EmptyPrivateKeyFile,
    /// The private-key reference is relative and could change meaning with process cwd.
    #[error("downstream TLS private_key_file must be an absolute path")]
    RelativePrivateKeyFile,
}

impl DownstreamTlsConfig {
    /// Validates deterministic TLS-reference invariants without reading secret material.
    ///
    /// File contents are intentionally materialized once by the Pingora delivery adapter directly
    /// before listener construction, avoiding a validate-then-reload time-of-check/time-of-use gap.
    pub fn validate(&self) -> Result<(), DownstreamTlsConfigError> {
        validate_absolute_path(
            &self.certificate_chain_file,
            DownstreamTlsConfigError::EmptyCertificateChainFile,
            DownstreamTlsConfigError::RelativeCertificateChainFile,
        )?;
        validate_absolute_path(
            &self.private_key_file,
            DownstreamTlsConfigError::EmptyPrivateKeyFile,
            DownstreamTlsConfigError::RelativePrivateKeyFile,
        )?;
        Ok(())
    }

    /// Returns the absolute certificate-chain reference supplied by the operator.
    pub fn certificate_chain_file(&self) -> &Path {
        &self.certificate_chain_file
    }

    /// Returns the absolute private-key reference supplied by the operator.
    pub fn private_key_file(&self) -> &Path {
        &self.private_key_file
    }

    /// Returns the explicit ALPN policy for the downstream listener.
    pub fn alpn(&self) -> DownstreamAlpnPolicy {
        self.alpn
    }
}

fn validate_absolute_path(
    path: &Path,
    empty_error: DownstreamTlsConfigError,
    relative_error: DownstreamTlsConfigError,
) -> Result<(), DownstreamTlsConfigError> {
    if path.as_os_str().is_empty() || path.to_string_lossy().trim().is_empty() {
        return Err(empty_error);
    }
    if !path.is_absolute() {
        return Err(relative_error);
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::process::{Command, Stdio};

    use tempfile::{tempdir, TempDir};

    use super::*;
    use crate::tls_delivery::{build_downstream_tls_settings, DownstreamTlsDeliveryError};

    fn direct_config(
        certificate_chain_file: PathBuf,
        private_key_file: PathBuf,
    ) -> DownstreamTlsConfig {
        DownstreamTlsConfig {
            certificate_chain_file,
            private_key_file,
            alpn: DownstreamAlpnPolicy::H2Http1,
        }
    }

    fn run_openssl(args: &[&str]) {
        let status = Command::new("openssl")
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("CI must provide the explicitly installed openssl CLI");
        assert!(status.success(), "openssl command failed: {args:?}");
    }

    fn issue_certificate() -> (TempDir, PathBuf, PathBuf) {
        let directory = tempdir().expect("certificate workspace should be available");
        let certificate = directory.path().join("server.crt");
        let private_key = directory.path().join("server.key");
        run_openssl(&[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            private_key.to_str().expect("UTF-8 private-key path"),
            "-out",
            certificate.to_str().expect("UTF-8 certificate path"),
            "-subj",
            "/CN=gateway.test",
            "-days",
            "1",
            "-sha256",
        ]);
        (directory, certificate, private_key)
    }

    fn delivery_error(config: &DownstreamTlsConfig) -> DownstreamTlsDeliveryError {
        build_downstream_tls_settings(config)
            .err()
            .expect("invalid TLS material must not become listener authority")
    }

    #[test]
    fn delivery_accepts_valid_material_and_accessors_preserve_operator_references() {
        let (_directory, certificate, private_key) = issue_certificate();
        let config = direct_config(certificate.clone(), private_key.clone());

        config
            .validate()
            .expect("absolute material references are valid");
        assert_eq!(config.certificate_chain_file(), certificate.as_path());
        assert_eq!(config.private_key_file(), private_key.as_path());
        assert_eq!(config.alpn(), DownstreamAlpnPolicy::H2Http1);
        build_downstream_tls_settings(&config)
            .expect("matching certificate/key material should produce Pingora TLS settings");
    }

    #[test]
    fn delivery_propagates_config_validation_before_materialization() {
        let config = direct_config(PathBuf::from("relative.crt"), PathBuf::from("/tmp/key.pem"));
        assert!(matches!(
            delivery_error(&config),
            DownstreamTlsDeliveryError::InvalidConfig(
                DownstreamTlsConfigError::RelativeCertificateChainFile
            )
        ));
    }

    #[test]
    fn delivery_rejects_non_utf8_material_references_without_lossy_rewrite() {
        let non_utf8_certificate =
            PathBuf::from(OsString::from_vec(b"/tmp/cwl-cert-\xff".to_vec()));
        let certificate_config = direct_config(non_utf8_certificate, PathBuf::from("/tmp/key.pem"));
        assert!(matches!(
            delivery_error(&certificate_config),
            DownstreamTlsDeliveryError::NonUtf8Path {
                field: "certificate_chain_file"
            }
        ));

        let non_utf8_key = PathBuf::from(OsString::from_vec(b"/tmp/cwl-key-\xff".to_vec()));
        let key_config = direct_config(PathBuf::from("/tmp/cert.pem"), non_utf8_key);
        assert!(matches!(
            delivery_error(&key_config),
            DownstreamTlsDeliveryError::NonUtf8Path {
                field: "private_key_file"
            }
        ));
    }

    #[test]
    fn delivery_reports_unreadable_and_mismatched_material_as_materialization_errors() {
        let (_directory, certificate, private_key) = issue_certificate();
        let missing_certificate = certificate.with_file_name("missing.crt");
        let missing_config = direct_config(missing_certificate, private_key.clone());
        assert!(matches!(
            delivery_error(&missing_config),
            DownstreamTlsDeliveryError::Materialization(_)
        ));

        let mismatch_directory = tempdir().expect("mismatch-key workspace should be available");
        let mismatched_key = mismatch_directory.path().join("mismatched.key");
        run_openssl(&[
            "genpkey",
            "-algorithm",
            "RSA",
            "-out",
            mismatched_key.to_str().expect("UTF-8 mismatched-key path"),
            "-pkeyopt",
            "rsa_keygen_bits:2048",
        ]);
        let mismatch_config = direct_config(certificate, mismatched_key);
        assert!(matches!(
            delivery_error(&mismatch_config),
            DownstreamTlsDeliveryError::Materialization(_)
        ));
    }
}

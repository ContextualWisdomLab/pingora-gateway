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

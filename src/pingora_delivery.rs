//! Pingora delivery adapter for validated edge contracts.
//!
//! This module is intentionally the only place where the transport-neutral edge contract is
//! converted into Pingora connection objects. Consumer products should depend on the contract,
//! not on Pingora types.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use pingora::tls::x509::X509;
use pingora::upstreams::peer::{HttpPeer, HttpUpstreamRequestPolicy, ALPN};
use thiserror::Error;

use crate::edge_contract::{GatewayConfigError, UpstreamConfig};

/// Failures while translating an admitted edge contract into Pingora transport authority.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PeerBuildError {
    /// The transport-neutral upstream contract itself is invalid.
    #[error("invalid upstream configuration: {0}")]
    InvalidConfiguration(#[from] GatewayConfigError),
    /// The configured trust bundle cannot be read before network authority is granted.
    #[error("unable to read TLS trust bundle {path:?}: {kind:?}")]
    ReadTrustBundle {
        /// Operator-selected absolute trust-bundle path.
        path: PathBuf,
        /// Stable operating-system error category.
        kind: ErrorKind,
    },
    /// The configured trust bundle is not a usable PEM certificate bundle.
    #[error("invalid TLS trust bundle {path:?}: {reason}")]
    InvalidTrustBundle {
        /// Operator-selected absolute trust-bundle path.
        path: PathBuf,
        /// Stable diagnostic without embedding certificate contents.
        reason: String,
    },
}

/// Builds a Pingora upstream peer from one validated edge-contract upstream.
///
/// The adapter revalidates the upstream so callers cannot bypass fail-closed contract checks by
/// constructing an [`UpstreamConfig`] directly. TLS certificate and hostname verification remain
/// enabled explicitly, HTTP/1.1 is the only accepted upstream protocol in the initial contract,
/// and every timeout comes from the versioned configuration rather than a hidden default.
pub fn build_peer(upstream: &UpstreamConfig) -> Result<HttpPeer, PeerBuildError> {
    upstream.validate()?;
    build_peer_from_validated(upstream)
}

/// Constructs Pingora transport state after the enclosing gateway contract has already validated
/// this upstream.
///
/// Configuration semantics have already been checked, but an explicit trust bundle still requires
/// fail-closed filesystem and PEM loading before listeners are opened.
pub(crate) fn build_peer_from_validated(
    upstream: &UpstreamConfig,
) -> Result<HttpPeer, PeerBuildError> {
    let mut peer = HttpPeer::new(
        upstream.address,
        upstream.tls,
        upstream.sni.clone().unwrap_or_default(),
    );

    peer.options.verify_cert = true;
    peer.options.verify_hostname = true;
    peer.options.alpn = ALPN::H1;
    peer.options.connection_timeout = Some(Duration::from_millis(upstream.timeouts.connection_ms));
    peer.options.total_connection_timeout =
        Some(Duration::from_millis(upstream.timeouts.total_connection_ms));
    peer.options.read_timeout = Some(Duration::from_millis(upstream.timeouts.read_ms));
    peer.options.write_timeout = Some(Duration::from_millis(upstream.timeouts.write_ms));
    peer.options.idle_timeout = Some(Duration::from_millis(upstream.timeouts.idle_ms));
    peer.options.http_upstream_request_policy = HttpUpstreamRequestPolicy::standard();

    if let Some(path) = upstream.trust_bundle_file.as_deref() {
        peer.options.ca = Some(Arc::new(load_trust_bundle(path)?));
    }

    Ok(peer)
}

fn load_trust_bundle(path: &Path) -> Result<Box<[X509]>, PeerBuildError> {
    let source = fs::read(path).map_err(|error| PeerBuildError::ReadTrustBundle {
        path: path.to_path_buf(),
        kind: error.kind(),
    })?;
    let certificates =
        X509::stack_from_pem(&source).map_err(|error| PeerBuildError::InvalidTrustBundle {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
    require_certificates(path, certificates)
}

fn require_certificates(
    path: &Path,
    certificates: Vec<X509>,
) -> Result<Box<[X509]>, PeerBuildError> {
    if certificates.is_empty() {
        return Err(PeerBuildError::InvalidTrustBundle {
            path: path.to_path_buf(),
            reason: "bundle contains no certificates".to_string(),
        });
    }
    Ok(certificates.into_boxed_slice())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge_contract::UpstreamTimeouts;

    fn cleartext_upstream_without_custom_roots() -> UpstreamConfig {
        UpstreamConfig {
            name: "origin".to_string(),
            address: "127.0.0.1:8080"
                .parse()
                .expect("loopback upstream must parse"),
            tls: false,
            sni: None,
            trust_bundle_file: None,
            timeouts: UpstreamTimeouts {
                connection_ms: 100,
                total_connection_ms: 200,
                read_ms: 300,
                write_ms: 400,
                idle_ms: 500,
            },
        }
    }

    #[test]
    fn peer_without_custom_trust_bundle_preserves_platform_roots() {
        let peer = build_peer(&cleartext_upstream_without_custom_roots())
            .expect("valid cleartext upstream should build a peer");

        assert!(peer.options.ca.is_none());
    }

    #[test]
    fn parsed_trust_bundle_must_contain_a_certificate() {
        let path = Path::new("/tmp/empty-trust-bundle.pem");

        assert_eq!(
            require_certificates(path, Vec::new()).unwrap_err(),
            PeerBuildError::InvalidTrustBundle {
                path: path.to_path_buf(),
                reason: "bundle contains no certificates".to_string(),
            }
        );
    }

    #[test]
    fn parsed_trust_bundle_preserves_nonempty_certificate_stack() {
        let path = Path::new("/tmp/nonempty-trust-bundle.pem");
        let certificate = X509::builder()
            .expect("OpenSSL should allocate an X509 builder")
            .build();

        let certificates = require_certificates(path, vec![certificate])
            .expect("a nonempty parsed certificate stack must be accepted");

        assert_eq!(certificates.len(), 1);
    }
}

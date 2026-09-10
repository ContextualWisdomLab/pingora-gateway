//! Pingora delivery adapter for the transport-neutral downstream TLS contract.
//!
//! The adapter consumes certificate and key references exactly once immediately before listener
//! construction. It does not issue, rotate, persist or otherwise become authoritative for TLS
//! identity material.

use std::fmt::Display;

use pingora::listeners::tls::TlsSettings;
use pingora::tls::ssl::{AlpnError, SslVersion};
use thiserror::Error;

use crate::downstream_tls::{DownstreamAlpnPolicy, DownstreamTlsConfig, DownstreamTlsConfigError};

const H2_ALPN: &[u8] = b"h2";
const HTTP1_ALPN: &[u8] = b"http/1.1";

/// TLS 1.2 compatibility ciphers admitted by the versioned downstream HTTPS/H2 profile.
///
/// Only ephemeral ECDHE key exchange with AEAD is retained. Static RSA, static ECDH and finite-
/// field DH suites are excluded so TLS 1.2 compatibility does not reintroduce key exchanges
/// deprecated by RFC 10015.
const DOWNSTREAM_TLS12_CIPHER_LIST: &str = "ECDHE-ECDSA-AES128-GCM-SHA256\
:ECDHE-RSA-AES128-GCM-SHA256\
:ECDHE-ECDSA-AES256-GCM-SHA384\
:ECDHE-RSA-AES256-GCM-SHA384\
:ECDHE-ECDSA-CHACHA20-POLY1305\
:ECDHE-RSA-CHACHA20-POLY1305";

/// TLS 1.3 AEAD suites explicitly admitted by the downstream edge profile.
const DOWNSTREAM_TLS13_CIPHERSUITES: &str =
    "TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256";

/// Reasons validated downstream TLS configuration cannot be materialized for Pingora.
#[derive(Debug, Error)]
pub enum DownstreamTlsDeliveryError {
    /// The transport-neutral TLS declaration is invalid.
    #[error(transparent)]
    InvalidConfig(#[from] DownstreamTlsConfigError),
    /// A configured filesystem reference cannot be represented by Pingora's path API.
    #[error("downstream TLS {field} path is not valid UTF-8")]
    NonUtf8Path {
        /// Stable Admin Config field whose path cannot be represented.
        field: &'static str,
    },
    /// Pingora/OpenSSL could not load or validate the supplied certificate/key material.
    #[error("unable to materialize downstream TLS settings: {0}")]
    Materialization(String),
    /// Pingora/OpenSSL rejected the code-owned downstream TLS security profile.
    #[error("unable to apply downstream TLS security profile: {0}")]
    SecurityProfile(String),
}

fn security_profile_error(error: impl Display) -> DownstreamTlsDeliveryError {
    DownstreamTlsDeliveryError::SecurityProfile(error.to_string())
}

/// Selects the highest-preference protocol admitted by the `h2_http1` edge contract.
///
/// Pingora 0.9.0's convenience `enable_h2()` callback returns `NOACK` when a client sends ALPN
/// but offers no supported protocol. RFC 7301 requires that case to terminate with the fatal
/// `no_application_protocol` alert. This adapter therefore uses Pingora's public TLS builder API
/// to preserve H2 preference/H1 fallback while making no-overlap and malformed ALPN fail closed.
fn select_h2_http1(client_protocols: &[u8]) -> Result<&[u8], AlpnError> {
    let mut remaining = client_protocols;
    let mut h2 = None;
    let mut http1 = None;

    while let Some((&length, rest)) = remaining.split_first() {
        let length = usize::from(length);
        if length == 0 || length > rest.len() {
            return Err(AlpnError::ALERT_FATAL);
        }
        let (protocol, tail) = rest.split_at(length);
        if protocol == H2_ALPN {
            h2 = Some(protocol);
        } else if protocol == HTTP1_ALPN {
            http1 = Some(protocol);
        }
        remaining = tail;
    }

    h2.or(http1).ok_or(AlpnError::ALERT_FATAL)
}

/// Applies the code-owned compatibility profile instead of inheriting supplier helper defaults.
///
/// This migration serves existing HTTPS/HTTP/2 clients, so TLS 1.2 remains an explicit floor and
/// TLS 1.3 the ceiling. TLS 1.2 is constrained to ephemeral ECDHE + AEAD; TLS 1.3 is constrained
/// to the current AES-GCM and ChaCha20-Poly1305 suites. Changing this profile is therefore a
/// versioned edge-security decision rather than an ambient OpenSSL/Pingora upgrade side effect.
fn apply_downstream_tls_security_profile(
    settings: &mut TlsSettings,
) -> Result<(), DownstreamTlsDeliveryError> {
    settings
        .set_min_proto_version(Some(SslVersion::TLS1_2))
        .map_err(security_profile_error)?;
    settings
        .set_max_proto_version(Some(SslVersion::TLS1_3))
        .map_err(security_profile_error)?;
    settings
        .set_cipher_list(DOWNSTREAM_TLS12_CIPHER_LIST)
        .map_err(security_profile_error)?;
    settings
        .set_ciphersuites(DOWNSTREAM_TLS13_CIPHERSUITES)
        .map_err(security_profile_error)?;
    Ok(())
}

/// Builds one Pingora TLS listener configuration from validated operator references.
///
/// The returned settings negotiate only the protocol policy explicitly represented by the
/// contract. Current `H2Http1` semantics prefer HTTP/2, allow HTTP/1.1 fallback when offered,
/// and fail the handshake when an ALPN-bearing client offers no admitted protocol; h2c and
/// HTTP/3 are deliberately absent. Protocol versions and cipher suites are also pinned by the
/// code-owned downstream TLS security profile rather than inherited from supplier defaults.
pub fn build_downstream_tls_settings(
    config: &DownstreamTlsConfig,
) -> Result<TlsSettings, DownstreamTlsDeliveryError> {
    config.validate()?;
    let certificate_chain_file = config.certificate_chain_file().to_str().ok_or(
        DownstreamTlsDeliveryError::NonUtf8Path {
            field: "certificate_chain_file",
        },
    )?;
    let private_key_file =
        config
            .private_key_file()
            .to_str()
            .ok_or(DownstreamTlsDeliveryError::NonUtf8Path {
                field: "private_key_file",
            })?;

    let mut settings = TlsSettings::intermediate(certificate_chain_file, private_key_file)
        .map_err(|error| DownstreamTlsDeliveryError::Materialization(error.to_string()))?;
    settings
        .check_private_key()
        .map_err(|error| DownstreamTlsDeliveryError::Materialization(error.to_string()))?;
    apply_downstream_tls_security_profile(&mut settings)?;

    match config.alpn() {
        DownstreamAlpnPolicy::H2Http1 => {
            settings.set_alpn_select_callback(|_, alpn_in| select_h2_http1(alpn_in));
        }
    }
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::{security_profile_error, select_h2_http1, H2_ALPN, HTTP1_ALPN};

    #[test]
    fn security_profile_errors_preserve_backend_diagnostics() {
        assert_eq!(
            security_profile_error("profile rejected").to_string(),
            "unable to apply downstream TLS security profile: profile rejected"
        );
    }

    #[test]
    fn strict_alpn_prefers_h2_even_when_http1_is_offered_first() {
        assert_eq!(
            select_h2_http1(b"\x08http/1.1\x02h2").expect("h2 should be selected"),
            H2_ALPN
        );
    }

    #[test]
    fn strict_alpn_selects_http1_when_h2_is_absent() {
        assert_eq!(
            select_h2_http1(b"\x08http/1.1").expect("HTTP/1.1 should be selected"),
            HTTP1_ALPN
        );
    }

    #[test]
    fn strict_alpn_rejects_no_overlap() {
        assert!(select_h2_http1(b"\x03foo").is_err());
        assert!(select_h2_http1(&[]).is_err());
    }

    #[test]
    fn strict_alpn_rejects_malformed_wire_lists_without_panicking() {
        assert!(select_h2_http1(b"\x00").is_err());
        assert!(select_h2_http1(b"\x08http").is_err());
        assert!(select_h2_http1(b"\x02h2\x08http").is_err());
        assert!(select_h2_http1(b"\x08http/1.1\x00").is_err());
    }
}

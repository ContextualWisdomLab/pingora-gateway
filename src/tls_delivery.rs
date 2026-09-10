//! Pingora delivery adapter for the transport-neutral downstream TLS contract.
//!
//! The adapter consumes certificate and key references exactly once immediately before listener
//! construction. It does not issue, rotate, persist or otherwise become authoritative for TLS
//! identity material.

use pingora::listeners::tls::TlsSettings;
use pingora::tls::ssl::AlpnError;
use thiserror::Error;

use crate::downstream_tls::{DownstreamAlpnPolicy, DownstreamTlsConfig, DownstreamTlsConfigError};

const H2_ALPN: &[u8] = b"h2";
const HTTP1_ALPN: &[u8] = b"http/1.1";

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
}

/// Selects the highest-preference protocol admitted by the `h2_http1` edge contract.
///
/// Pingora 0.9.0's convenience `enable_h2()` callback returns `NOACK` when a client sends ALPN
/// but offers no supported protocol. RFC 7301 requires that case to terminate with the fatal
/// `no_application_protocol` alert. This adapter therefore uses Pingora's public TLS builder API
/// to preserve H2 preference/H1 fallback while making no-overlap and malformed ALPN fail closed.
fn select_h2_http1<'a>(client_protocols: &'a [u8]) -> Result<&'a [u8], AlpnError> {
    let mut remaining = client_protocols;
    let mut http1 = None;

    while let Some((&length, rest)) = remaining.split_first() {
        let length = usize::from(length);
        if length == 0 || length > rest.len() {
            return Err(AlpnError::ALERT_FATAL);
        }
        let (protocol, tail) = rest.split_at(length);
        if protocol == H2_ALPN {
            return Ok(protocol);
        }
        if protocol == HTTP1_ALPN {
            http1 = Some(protocol);
        }
        remaining = tail;
    }

    http1.ok_or(AlpnError::ALERT_FATAL)
}

/// Builds one Pingora TLS listener configuration from validated operator references.
///
/// The returned settings negotiate only the protocol policy explicitly represented by the
/// contract. Current `H2Http1` semantics prefer HTTP/2, allow HTTP/1.1 fallback when offered,
/// and fail the handshake when an ALPN-bearing client offers no admitted protocol; h2c and
/// HTTP/3 are deliberately absent.
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

    match config.alpn() {
        DownstreamAlpnPolicy::H2Http1 => {
            settings.set_alpn_select_callback(|_, alpn_in| select_h2_http1(alpn_in));
        }
    }
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::{select_h2_http1, H2_ALPN, HTTP1_ALPN};

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
    }
}

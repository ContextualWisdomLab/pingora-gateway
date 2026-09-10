//! Pingora delivery adapter for the transport-neutral downstream TLS contract.
//!
//! The adapter consumes certificate and key references exactly once immediately before listener
//! construction. It does not issue, rotate, persist or otherwise become authoritative for TLS
//! identity material.

use pingora::listeners::tls::TlsSettings;
use thiserror::Error;

use crate::downstream_tls::{DownstreamAlpnPolicy, DownstreamTlsConfig, DownstreamTlsConfigError};

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

/// Builds one Pingora TLS listener configuration from validated operator references.
///
/// The returned settings negotiate only the protocol policy explicitly represented by the
/// contract. Current `H2Http1` semantics map to Pingora's H2-preferred/H1-allowed ALPN helper;
/// h2c and HTTP/3 are deliberately absent.
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
        DownstreamAlpnPolicy::H2Http1 => settings.enable_h2(),
    }
    Ok(settings)
}

//! Versioned, transport-neutral configuration contract for the shared edge runtime.
//!
//! The contract deliberately contains only values that the application boundary needs.
//! Pingora-specific transport types are constructed later by the delivery adapter so that
//! consumer products never depend on Pingora internals.

use std::collections::HashSet;
use std::net::SocketAddr;

use serde::Deserialize;
use thiserror::Error;

/// The only configuration version implemented by this release line.
pub const CURRENT_GATEWAY_CONFIG_VERSION: u32 = 1;

/// Fail-closed configuration for one gateway process.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GatewayConfig {
    /// Version of the public edge configuration contract.
    pub version: u32,
    /// Socket address on which the gateway accepts downstream connections.
    pub listener: SocketAddr,
    /// Explicit set of upstream services that the gateway may contact.
    pub upstreams: Vec<UpstreamConfig>,
}

/// An explicitly approved upstream target.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpstreamConfig {
    /// Stable operator-facing upstream name.
    pub name: String,
    /// Resolved socket endpoint; arbitrary per-request destinations are intentionally absent.
    pub address: SocketAddr,
    /// Whether the upstream connection must use TLS.
    #[serde(default)]
    pub tls: bool,
    /// TLS server name used for SNI and hostname verification.
    #[serde(default)]
    pub sni: Option<String>,
}

/// Reasons an edge configuration is not safe to activate.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GatewayConfigError {
    /// YAML could not be decoded into the strict contract.
    #[error("gateway configuration is not valid YAML for the current contract: {0}")]
    Parse(String),
    /// The configuration requests a contract version this binary does not implement.
    #[error("unsupported gateway configuration version {0}")]
    UnsupportedVersion(u32),
    /// At least one upstream is required so the proxy cannot start in an ambiguous state.
    #[error("gateway configuration must contain at least one upstream")]
    NoUpstreams,
    /// Upstream names are stable identifiers and therefore must be unique.
    #[error("duplicate upstream name: {upstream_name}")]
    DuplicateUpstreamName {
        /// Duplicate stable upstream identifier.
        upstream_name: String,
    },
    /// A TLS upstream must have an explicit server name for hostname verification.
    #[error("TLS upstream {upstream_name} requires an explicit SNI server name")]
    MissingTlsServerName {
        /// Upstream whose TLS identity is incomplete.
        upstream_name: String,
    },
    /// Empty or whitespace-only upstream names are not stable identifiers.
    #[error("upstream name must not be empty")]
    EmptyUpstreamName,
    /// SNI must contain a non-empty hostname when TLS is enabled.
    #[error("TLS upstream {upstream_name} has an empty SNI server name")]
    EmptyTlsServerName {
        /// Upstream whose TLS identity is incomplete.
        upstream_name: String,
    },
    /// Cleartext upstreams must not carry an unused TLS identity.
    #[error("cleartext upstream {upstream_name} must not define an SNI server name")]
    UnexpectedTlsServerName {
        /// Upstream that specified SNI while TLS is disabled.
        upstream_name: String,
    },
}

impl GatewayConfig {
    /// Parses and validates a gateway configuration without activating any network listener.
    ///
    /// Validation is intentionally separate from Pingora construction so configuration can be
    /// audited and tested deterministically before the runtime obtains network authority.
    pub fn from_yaml(input: &str) -> Result<Self, GatewayConfigError> {
        let config: Self =
            serde_yaml::from_str(input).map_err(|error| GatewayConfigError::Parse(error.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// Verifies all configuration invariants required before network authority is granted.
    pub fn validate(&self) -> Result<(), GatewayConfigError> {
        if self.version != CURRENT_GATEWAY_CONFIG_VERSION {
            return Err(GatewayConfigError::UnsupportedVersion(self.version));
        }
        if self.upstreams.is_empty() {
            return Err(GatewayConfigError::NoUpstreams);
        }

        let mut names = HashSet::with_capacity(self.upstreams.len());
        for upstream in &self.upstreams {
            let normalized_name = upstream.name.trim();
            if normalized_name.is_empty() {
                return Err(GatewayConfigError::EmptyUpstreamName);
            }
            if !names.insert(normalized_name) {
                return Err(GatewayConfigError::DuplicateUpstreamName {
                    upstream_name: normalized_name.to_string(),
                });
            }

            match (upstream.tls, upstream.sni.as_deref()) {
                (true, None) => {
                    return Err(GatewayConfigError::MissingTlsServerName {
                        upstream_name: normalized_name.to_string(),
                    });
                }
                (true, Some(server_name)) if server_name.trim().is_empty() => {
                    return Err(GatewayConfigError::EmptyTlsServerName {
                        upstream_name: normalized_name.to_string(),
                    });
                }
                (false, Some(_)) => {
                    return Err(GatewayConfigError::UnexpectedTlsServerName {
                        upstream_name: normalized_name.to_string(),
                    });
                }
                _ => {}
            }
        }

        Ok(())
    }
}

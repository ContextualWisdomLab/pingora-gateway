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
    /// Separate socket address that exposes only low-cardinality Prometheus metrics.
    pub metrics_listener: SocketAddr,
    /// Maximum request body admitted by this gateway process, in bytes.
    pub max_request_body_bytes: u64,
    /// Explicit set of upstream services that the gateway may contact.
    pub upstreams: Vec<UpstreamConfig>,
}

/// Explicit I/O budgets for one upstream connection.
///
/// The gateway intentionally has no hidden timeout defaults. Operators must choose budgets that
/// match the owning product's latency and failure contract, and every budget must be positive.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpstreamTimeouts {
    /// Maximum time allowed to establish one TCP/TLS connection attempt, in milliseconds.
    pub connection_ms: u64,
    /// Maximum total time across connection establishment attempts, in milliseconds.
    pub total_connection_ms: u64,
    /// Maximum time allowed while waiting for upstream response bytes, in milliseconds.
    pub read_ms: u64,
    /// Maximum time allowed while writing request bytes upstream, in milliseconds.
    pub write_ms: u64,
    /// Maximum reusable upstream-connection idle time, in milliseconds.
    pub idle_ms: u64,
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
    /// Explicit connection and I/O budgets for this upstream.
    pub timeouts: UpstreamTimeouts,
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
    /// Traffic and metrics endpoints must never compete for the same socket authority.
    #[error("listener and metrics_listener must use distinct socket addresses")]
    ListenerCollision,
    /// A zero request-body limit would reject every body and is almost certainly misconfiguration.
    #[error("max_request_body_bytes must be greater than zero")]
    InvalidRequestBodyLimit,
    /// At least one upstream is required so the proxy cannot start in an ambiguous state.
    #[error("gateway configuration must contain at least one upstream")]
    NoUpstreams,
    /// Version 1 deliberately has no routing or load-balancing semantics, so one target is allowed.
    #[error("gateway configuration version 1 requires exactly one upstream; received {actual}")]
    UnsupportedUpstreamCount {
        /// Number of explicit upstream authorities presented by the configuration.
        actual: usize,
    },
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
    /// Timeout budgets must be positive so the runtime never silently acquires an infinite budget.
    #[error("upstream {upstream_name} has invalid zero timeout budget {timeout_name}")]
    InvalidTimeoutBudget {
        /// Upstream with the invalid timeout.
        upstream_name: String,
        /// Stable edge-contract field name whose value is invalid.
        timeout_name: &'static str,
    },
}

impl GatewayConfig {
    /// Parses and validates a gateway configuration without activating any network listener.
    ///
    /// Validation is intentionally separate from Pingora construction so configuration can be
    /// audited and tested deterministically before the runtime obtains network authority.
    pub fn from_yaml(input: &str) -> Result<Self, GatewayConfigError> {
        let config: Self = serde_yaml::from_str(input)
            .map_err(|error| GatewayConfigError::Parse(error.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// Verifies all configuration invariants required before network authority is granted.
    pub fn validate(&self) -> Result<(), GatewayConfigError> {
        if self.version != CURRENT_GATEWAY_CONFIG_VERSION {
            return Err(GatewayConfigError::UnsupportedVersion(self.version));
        }
        if self.listener == self.metrics_listener {
            return Err(GatewayConfigError::ListenerCollision);
        }
        if self.max_request_body_bytes == 0 {
            return Err(GatewayConfigError::InvalidRequestBodyLimit);
        }
        if self.upstreams.is_empty() {
            return Err(GatewayConfigError::NoUpstreams);
        }

        let mut names = HashSet::with_capacity(self.upstreams.len());
        for upstream in &self.upstreams {
            upstream.validate()?;
            let normalized_name = upstream.name.trim();
            if !names.insert(normalized_name) {
                return Err(GatewayConfigError::DuplicateUpstreamName {
                    upstream_name: normalized_name.to_string(),
                });
            }
        }

        if self.upstreams.len() != 1 {
            return Err(GatewayConfigError::UnsupportedUpstreamCount {
                actual: self.upstreams.len(),
            });
        }

        Ok(())
    }
}

impl UpstreamConfig {
    /// Validates the invariants required before this upstream can become network authority.
    pub fn validate(&self) -> Result<(), GatewayConfigError> {
        let normalized_name = self.name.trim();
        if normalized_name.is_empty() {
            return Err(GatewayConfigError::EmptyUpstreamName);
        }

        match (self.tls, self.sni.as_deref()) {
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

        for (timeout_name, timeout_value) in [
            ("connection_ms", self.timeouts.connection_ms),
            ("total_connection_ms", self.timeouts.total_connection_ms),
            ("read_ms", self.timeouts.read_ms),
            ("write_ms", self.timeouts.write_ms),
            ("idle_ms", self.timeouts.idle_ms),
        ] {
            if timeout_value == 0 {
                return Err(GatewayConfigError::InvalidTimeoutBudget {
                    upstream_name: normalized_name.to_string(),
                    timeout_name,
                });
            }
        }

        Ok(())
    }
}

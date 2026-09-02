//! Versioned, transport-neutral configuration contract for the shared edge runtime.
//!
//! The contract deliberately contains only values that the application boundary needs.
//! Pingora-specific transport types are constructed later by the delivery adapter so that
//! consumer products never depend on Pingora internals.

use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

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
    /// Maximum number of non-health downstream requests admitted concurrently by this process.
    pub max_in_flight_requests: usize,
    /// Maximum number of reusable upstream keepalive connections retained by Pingora.
    pub upstream_keepalive_pool_size: usize,
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
    /// Optional absolute PEM bundle of additional trust anchors for this TLS upstream.
    ///
    /// The gateway consumes trust roots supplied by the operator; it does not issue, rotate, or
    /// otherwise become authoritative for certificates. When omitted, Pingora uses platform roots.
    #[serde(default)]
    pub trust_bundle_file: Option<PathBuf>,
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
    /// Port zero would delegate the traffic listener to an ephemeral OS-selected authority.
    #[error("listener must use a non-zero port")]
    ZeroListenerPort,
    /// Port zero would make the declared metrics endpoint indeterminate.
    #[error("metrics_listener must use a non-zero port")]
    ZeroMetricsListenerPort,
    /// Traffic and metrics endpoints must never overlap the same effective socket authority.
    #[error("listener and metrics_listener socket authorities must not overlap")]
    ListenerCollision,
    /// An upstream must not resolve back to the gateway's downstream traffic listener.
    #[error("upstream {upstream_name} socket authority must not overlap listener")]
    UpstreamListenerCollision {
        /// Stable upstream whose transport authority overlaps the traffic listener.
        upstream_name: String,
    },
    /// Application traffic must not resolve to the gateway's internal metrics listener.
    #[error("upstream {upstream_name} socket authority must not overlap metrics_listener")]
    UpstreamMetricsListenerCollision {
        /// Stable upstream whose transport authority overlaps the metrics listener.
        upstream_name: String,
    },
    /// An approved upstream must identify a concrete, connectable transport port.
    #[error("upstream {upstream_name} must use a non-zero port")]
    ZeroUpstreamPort {
        /// Stable upstream whose transport binding used port zero.
        upstream_name: String,
    },
    /// A zero request-body limit would reject every body and is almost certainly misconfiguration.
    #[error("max_request_body_bytes must be greater than zero")]
    InvalidRequestBodyLimit,
    /// A zero in-flight budget would reject every proxied request.
    #[error("max_in_flight_requests must be greater than zero")]
    InvalidInFlightRequestLimit,
    /// A zero keepalive pool silently disables reusable upstream connections and changes capacity.
    #[error("upstream_keepalive_pool_size must be greater than zero")]
    InvalidUpstreamKeepalivePoolSize,
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
    /// An explicitly configured trust bundle must identify a concrete filesystem path.
    #[error("TLS upstream {upstream_name} has an empty trust_bundle_file path")]
    EmptyTrustBundlePath {
        /// Upstream whose trust-bundle path is empty.
        upstream_name: String,
    },
    /// Trust-bundle paths are absolute so process working-directory changes cannot alter authority.
    #[error("TLS upstream {upstream_name} trust_bundle_file must be an absolute path")]
    RelativeTrustBundlePath {
        /// Upstream whose trust-bundle path is relative.
        upstream_name: String,
    },
    /// Cleartext upstreams must not carry unused certificate trust configuration.
    #[error("cleartext upstream {upstream_name} must not define trust_bundle_file")]
    UnexpectedTrustBundle {
        /// Upstream that specified a trust bundle while TLS is disabled.
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
        if self.listener.port() == 0 {
            return Err(GatewayConfigError::ZeroListenerPort);
        }
        if self.metrics_listener.port() == 0 {
            return Err(GatewayConfigError::ZeroMetricsListenerPort);
        }
        if socket_authorities_overlap(self.listener, self.metrics_listener) {
            return Err(GatewayConfigError::ListenerCollision);
        }
        if self.max_request_body_bytes == 0 {
            return Err(GatewayConfigError::InvalidRequestBodyLimit);
        }
        if self.max_in_flight_requests == 0 {
            return Err(GatewayConfigError::InvalidInFlightRequestLimit);
        }
        if self.upstream_keepalive_pool_size == 0 {
            return Err(GatewayConfigError::InvalidUpstreamKeepalivePoolSize);
        }
        if self.upstreams.is_empty() {
            return Err(GatewayConfigError::NoUpstreams);
        }

        let mut names = HashSet::with_capacity(self.upstreams.len());
        for upstream in &self.upstreams {
            upstream.validate()?;
            validate_upstream_authority_separation(
                self.listener,
                self.metrics_listener,
                upstream,
            )?;
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

pub(crate) fn socket_authorities_overlap(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
) -> bool {
    if listener.port() != metrics_listener.port() {
        return false;
    }

    match (listener.ip(), metrics_listener.ip()) {
        (IpAddr::V4(listener_ip), IpAddr::V4(metrics_ip)) => {
            listener_ip == metrics_ip || listener_ip.is_unspecified() || metrics_ip.is_unspecified()
        }
        (IpAddr::V6(listener_ip), IpAddr::V6(metrics_ip)) => {
            listener_ip == metrics_ip || listener_ip.is_unspecified() || metrics_ip.is_unspecified()
        }
        (IpAddr::V6(ipv6), IpAddr::V4(_)) | (IpAddr::V4(_), IpAddr::V6(ipv6)) => {
            // An IPv6 wildcard may also consume the IPv4 port on dual-stack platforms when
            // IPV6_V6ONLY is disabled. Reject the platform-dependent authority before activation.
            ipv6.is_unspecified()
        }
    }
}

pub(crate) fn validate_upstream_authority_separation(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    upstream: &UpstreamConfig,
) -> Result<(), GatewayConfigError> {
    let upstream_name = upstream.name.trim().to_string();
    if socket_authorities_overlap(listener, upstream.address) {
        return Err(GatewayConfigError::UpstreamListenerCollision { upstream_name });
    }
    if socket_authorities_overlap(metrics_listener, upstream.address) {
        return Err(GatewayConfigError::UpstreamMetricsListenerCollision { upstream_name });
    }
    Ok(())
}

impl UpstreamConfig {
    /// Validates the invariants required before this upstream can become network authority.
    pub fn validate(&self) -> Result<(), GatewayConfigError> {
        let normalized_name = self.name.trim();
        if normalized_name.is_empty() {
            return Err(GatewayConfigError::EmptyUpstreamName);
        }
        if self.address.port() == 0 {
            return Err(GatewayConfigError::ZeroUpstreamPort {
                upstream_name: normalized_name.to_string(),
            });
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

        if let Some(path) = self.trust_bundle_file.as_ref() {
            if path.as_os_str().is_empty() || path.to_string_lossy().trim().is_empty() {
                return Err(GatewayConfigError::EmptyTrustBundlePath {
                    upstream_name: normalized_name.to_string(),
                });
            }
            if !path.is_absolute() {
                return Err(GatewayConfigError::RelativeTrustBundlePath {
                    upstream_name: normalized_name.to_string(),
                });
            }
            if !self.tls {
                return Err(GatewayConfigError::UnexpectedTrustBundle {
                    upstream_name: normalized_name.to_string(),
                });
            }
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

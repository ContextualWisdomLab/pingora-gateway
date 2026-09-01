//! Runtime Configuration supporting context.
//!
//! YAML is parsed into a versioned contract and compiled into domain values before
//! Pingora starts. Unknown fields, ambiguous routes, unsafe listeners and implicit
//! private-network upstreams fail closed.

use std::{
    fs,
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    path::Path,
    time::Duration,
};

use serde::Deserialize;
use thiserror::Error;
use url::Url;

use crate::edge_routing::{Route, RouteId, RouteTable, RoutingError, UpstreamScheme, UpstreamTarget};

const MAX_ROUTES: usize = 256;
const MAX_BODY_BYTES_CEILING: u64 = 512 * 1024 * 1024;
const MAX_HEADER_BYTES_CEILING: usize = 256 * 1024;
const MAX_HEADER_COUNT_CEILING: usize = 512;
const MAX_TIMEOUT_MS: u64 = 120_000;

/// Validated request and upstream resource limits.
#[derive(Clone, Debug)]
pub struct Limits {
    /// Maximum downstream header fields.
    pub max_header_count: usize,
    /// Maximum aggregate downstream header name/value bytes.
    pub max_header_bytes: usize,
    /// Maximum streamed downstream request body bytes.
    pub max_body_bytes: u64,
    /// TCP connection timeout to an upstream.
    pub connect_timeout: Duration,
    /// Per-read upstream timeout.
    pub read_timeout: Duration,
    /// Per-write upstream timeout.
    pub write_timeout: Duration,
}

/// Fully validated startup configuration.
#[derive(Clone, Debug)]
pub struct ValidatedConfig {
    /// Non-privileged listener address.
    pub listener: SocketAddr,
    /// Deterministic routing aggregate.
    pub routes: RouteTable,
    /// Bounded resource policy.
    pub limits: Limits,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    version: u16,
    listener: String,
    limits: RawLimits,
    routes: Vec<RawRoute>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLimits {
    max_header_count: usize,
    max_header_bytes: usize,
    max_body_bytes: u64,
    connect_timeout_ms: u64,
    read_timeout_ms: u64,
    write_timeout_ms: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRoute {
    id: String,
    host: Option<String>,
    path_prefix: String,
    upstream: String,
    #[serde(default)]
    allow_private_networks: bool,
}

/// Configuration failure that prevents listener startup.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// File could not be read.
    #[error("cannot read gateway configuration: {0}")]
    Io(#[from] std::io::Error),
    /// YAML violated the versioned schema.
    #[error("invalid gateway YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
    /// Only version 1 is accepted by this binary.
    #[error("unsupported configuration version {0}; expected 1")]
    UnsupportedVersion(u16),
    /// Listener must be numeric, non-privileged and explicit.
    #[error("invalid listener: {0}")]
    InvalidListener(String),
    /// Resource limit is zero or exceeds the supported safety ceiling.
    #[error("invalid limit: {0}")]
    InvalidLimit(String),
    /// Route count is empty or exceeds its bound.
    #[error("route count must be between 1 and {MAX_ROUTES}")]
    InvalidRouteCount,
    /// Route matcher is malformed.
    #[error("invalid route {route_id}: {reason}")]
    InvalidRoute {
        /// Route identifier from configuration.
        route_id: String,
        /// Validation reason.
        reason: String,
    },
    /// Upstream URL or DNS resolution failed closed.
    #[error("invalid upstream for route {route_id}: {reason}")]
    InvalidUpstream {
        /// Route identifier from configuration.
        route_id: String,
        /// Validation reason.
        reason: String,
    },
    /// Routing aggregate rejected ambiguity.
    #[error(transparent)]
    Routing(#[from] RoutingError),
}

impl ValidatedConfig {
    /// Load and validate the complete startup contract before binding a socket.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        Self::from_yaml(&content)
    }

    /// Validate YAML content. Exposed for deterministic tests and config doctoring.
    pub fn from_yaml(content: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig = serde_yaml::from_str(content)?;
        if raw.version != 1 {
            return Err(ConfigError::UnsupportedVersion(raw.version));
        }
        let listener: SocketAddr = raw
            .listener
            .parse()
            .map_err(|_| ConfigError::InvalidListener(raw.listener.clone()))?;
        if listener.port() < 1024 || listener.ip().is_unspecified() && listener.port() == 0 {
            return Err(ConfigError::InvalidListener(raw.listener));
        }
        validate_limits(&raw.limits)?;
        if raw.routes.is_empty() || raw.routes.len() > MAX_ROUTES {
            return Err(ConfigError::InvalidRouteCount);
        }

        let routes = raw
            .routes
            .into_iter()
            .map(compile_route)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            listener,
            routes: RouteTable::new(routes)?,
            limits: Limits {
                max_header_count: raw.limits.max_header_count,
                max_header_bytes: raw.limits.max_header_bytes,
                max_body_bytes: raw.limits.max_body_bytes,
                connect_timeout: Duration::from_millis(raw.limits.connect_timeout_ms),
                read_timeout: Duration::from_millis(raw.limits.read_timeout_ms),
                write_timeout: Duration::from_millis(raw.limits.write_timeout_ms),
            },
        })
    }
}

fn validate_limits(limits: &RawLimits) -> Result<(), ConfigError> {
    if limits.max_header_count == 0 || limits.max_header_count > MAX_HEADER_COUNT_CEILING {
        return Err(ConfigError::InvalidLimit("max_header_count".to_owned()));
    }
    if limits.max_header_bytes == 0 || limits.max_header_bytes > MAX_HEADER_BYTES_CEILING {
        return Err(ConfigError::InvalidLimit("max_header_bytes".to_owned()));
    }
    if limits.max_body_bytes == 0 || limits.max_body_bytes > MAX_BODY_BYTES_CEILING {
        return Err(ConfigError::InvalidLimit("max_body_bytes".to_owned()));
    }
    for (name, value) in [
        ("connect_timeout_ms", limits.connect_timeout_ms),
        ("read_timeout_ms", limits.read_timeout_ms),
        ("write_timeout_ms", limits.write_timeout_ms),
    ] {
        if value == 0 || value > MAX_TIMEOUT_MS {
            return Err(ConfigError::InvalidLimit(name.to_owned()));
        }
    }
    Ok(())
}

fn compile_route(raw: RawRoute) -> Result<Route, ConfigError> {
    let id = RouteId::new(raw.id.clone())?;
    if !raw.path_prefix.starts_with('/') || raw.path_prefix.contains(['?', '#']) {
        return Err(ConfigError::InvalidRoute {
            route_id: raw.id,
            reason: "path_prefix must be an absolute path without query or fragment".to_owned(),
        });
    }
    let host = raw.host.map(|value| value.trim_end_matches('.').to_ascii_lowercase());
    if host.as_ref().is_some_and(|value| value.is_empty() || value.contains(['/', '@'])) {
        return Err(ConfigError::InvalidRoute {
            route_id: id.as_str().to_owned(),
            reason: "host must be a DNS host without port or userinfo".to_owned(),
        });
    }

    let parsed = Url::parse(&raw.upstream).map_err(|error| ConfigError::InvalidUpstream {
        route_id: id.as_str().to_owned(),
        reason: error.to_string(),
    })?;
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.path() != "/"
    {
        return Err(ConfigError::InvalidUpstream {
            route_id: id.as_str().to_owned(),
            reason: "upstream must be an origin URL only; credentials/path/query/fragment are forbidden".to_owned(),
        });
    }
    let scheme = match parsed.scheme() {
        "http" => UpstreamScheme::Http,
        "https" => UpstreamScheme::Https,
        other => {
            return Err(ConfigError::InvalidUpstream {
                route_id: id.as_str().to_owned(),
                reason: format!("unsupported scheme {other}"),
            });
        }
    };
    let upstream_host = parsed.host_str().ok_or_else(|| ConfigError::InvalidUpstream {
        route_id: id.as_str().to_owned(),
        reason: "upstream host is required".to_owned(),
    })?;
    let port = parsed.port_or_known_default().ok_or_else(|| ConfigError::InvalidUpstream {
        route_id: id.as_str().to_owned(),
        reason: "upstream port is required".to_owned(),
    })?;
    let resolved = (upstream_host, port)
        .to_socket_addrs()
        .map_err(|error| ConfigError::InvalidUpstream {
            route_id: id.as_str().to_owned(),
            reason: format!("DNS resolution failed: {error}"),
        })?
        .collect::<Vec<_>>();
    if resolved.is_empty() {
        return Err(ConfigError::InvalidUpstream {
            route_id: id.as_str().to_owned(),
            reason: "DNS resolution returned no addresses".to_owned(),
        });
    }
    if !raw.allow_private_networks && resolved.iter().any(|addr| is_non_public(addr.ip())) {
        return Err(ConfigError::InvalidUpstream {
            route_id: id.as_str().to_owned(),
            reason: "private, loopback, link-local, multicast or unspecified addresses require allow_private_networks: true".to_owned(),
        });
    }
    let socket = *resolved.first().ok_or_else(|| ConfigError::InvalidUpstream {
        route_id: id.as_str().to_owned(),
        reason: "DNS resolution returned no usable address".to_owned(),
    })?;
    let default_port = matches!(scheme, UpstreamScheme::Http) && port == 80
        || matches!(scheme, UpstreamScheme::Https) && port == 443;
    let authority = if default_port {
        upstream_host.to_owned()
    } else {
        format!("{upstream_host}:{port}")
    };

    Ok(Route {
        id,
        host,
        path_prefix: raw.path_prefix,
        upstream: UpstreamTarget {
            socket,
            host: upstream_host.to_owned(),
            authority,
            scheme,
        },
    })
}

fn is_non_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local() || ip.is_multicast() || ip.is_unspecified(),
        IpAddr::V6(ip) => ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() || ip.is_unique_local() || ip.is_unicast_link_local(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml(upstream: &str, allow_private: bool) -> String {
        format!(
            "version: 1\nlistener: 127.0.0.1:18080\nlimits:\n  max_header_count: 64\n  max_header_bytes: 32768\n  max_body_bytes: 1048576\n  connect_timeout_ms: 1000\n  read_timeout_ms: 5000\n  write_timeout_ms: 5000\nroutes:\n  - id: app\n    path_prefix: /\n    upstream: {upstream}\n    allow_private_networks: {allow_private}\n"
        )
    }

    #[test]
    fn private_upstream_requires_explicit_opt_in() {
        let error = ValidatedConfig::from_yaml(&yaml("http://127.0.0.1:19090", false))
            .err()
            .unwrap_or_else(|| panic!("private upstream must be rejected"));
        assert!(matches!(error, ConfigError::InvalidUpstream { .. }));
        assert!(ValidatedConfig::from_yaml(&yaml("http://127.0.0.1:19090", true)).is_ok());
    }

    #[test]
    fn unknown_fields_fail_closed() {
        let input = format!("{}unexpected: true\n", yaml("http://127.0.0.1:19090", true));
        assert!(matches!(ValidatedConfig::from_yaml(&input), Err(ConfigError::Yaml(_))));
    }
}

//! Edge Routing bounded context.
//!
//! This module owns deterministic route matching and upstream identities. It has
//! no dependency on Pingora transport types.

use std::net::SocketAddr;

use thiserror::Error;

/// Stable identifier used for operator evidence and low-cardinality telemetry.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RouteId(String);

impl RouteId {
    /// Construct a route identifier after validating its bounded format.
    pub fn new(value: impl Into<String>) -> Result<Self, RoutingError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
        if !valid {
            return Err(RoutingError::InvalidRouteId(value));
        }
        Ok(Self(value))
    }

    /// Return the validated route identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Upstream protocol supported by the initial gateway vertical.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpstreamScheme {
    /// Cleartext HTTP, normally for explicitly approved private networks.
    Http,
    /// HTTPS with certificate and hostname verification.
    Https,
}

/// A resolved, request-independent upstream target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamTarget {
    /// Network endpoint resolved and validated when configuration is loaded.
    pub socket: SocketAddr,
    /// Host used for Host and TLS hostname verification.
    pub host: String,
    /// Host header authority, including a non-default port when necessary.
    pub authority: String,
    /// Upstream protocol.
    pub scheme: UpstreamScheme,
}

/// Route aggregate used by the delivery adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Route {
    /// Stable route identity.
    pub id: RouteId,
    /// Optional exact downstream host match, normalized to lowercase.
    pub host: Option<String>,
    /// Path prefix. Longest prefix wins among eligible routes.
    pub path_prefix: String,
    /// Validated static upstream.
    pub upstream: UpstreamTarget,
}

/// Deterministic route table aggregate.
#[derive(Clone, Debug)]
pub struct RouteTable {
    routes: Vec<Route>,
}

impl RouteTable {
    /// Build a route table and reject duplicate matchers.
    pub fn new(mut routes: Vec<Route>) -> Result<Self, RoutingError> {
        for (index, left) in routes.iter().enumerate() {
            if routes.iter().skip(index + 1).any(|right| {
                left.host == right.host && left.path_prefix == right.path_prefix
            }) {
                return Err(RoutingError::DuplicateMatcher {
                    host: left.host.clone(),
                    path_prefix: left.path_prefix.clone(),
                });
            }
        }
        routes.sort_by(|left, right| {
            right
                .host
                .is_some()
                .cmp(&left.host.is_some())
                .then_with(|| right.path_prefix.len().cmp(&left.path_prefix.len()))
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(Self { routes })
    }

    /// Select an exact-host/longest-prefix route without consulting request data for an upstream.
    pub fn find(&self, host: Option<&str>, path: &str) -> Option<&Route> {
        self.routes.iter().find(|route| {
            let host_matches = route
                .host
                .as_deref()
                .is_none_or(|required| host.is_some_and(|actual| actual.eq_ignore_ascii_case(required)));
            host_matches && path.starts_with(&route.path_prefix)
        })
    }

    /// Number of bounded routes loaded into the aggregate.
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Whether no routes are configured.
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }
}

/// Edge Routing invariant failure.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum RoutingError {
    /// Route identity was empty, too long, or contained unsafe characters.
    #[error("invalid route id: {0}")]
    InvalidRouteId(String),
    /// Two routes claimed the same matcher and therefore had ambiguous ownership.
    #[error("duplicate route matcher host={host:?} path_prefix={path_prefix}")]
    DuplicateMatcher {
        /// Duplicate exact host, or none for host-agnostic routes.
        host: Option<String>,
        /// Duplicate path prefix.
        path_prefix: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(id: &str, host: Option<&str>, prefix: &str) -> Route {
        Route {
            id: RouteId::new(id).unwrap_or_else(|error| panic!("test route id: {error}")),
            host: host.map(str::to_owned),
            path_prefix: prefix.to_owned(),
            upstream: UpstreamTarget {
                socket: "127.0.0.1:9000".parse().unwrap_or_else(|error| panic!("test socket: {error}")),
                host: "127.0.0.1".to_owned(),
                authority: "127.0.0.1:9000".to_owned(),
                scheme: UpstreamScheme::Http,
            },
        }
    }

    #[test]
    fn exact_host_and_longest_prefix_have_precedence() {
        let table = RouteTable::new(vec![
            route("fallback", None, "/"),
            route("api", Some("app.example"), "/api/"),
            route("host-root", Some("app.example"), "/"),
        ])
        .unwrap_or_else(|error| panic!("route table: {error}"));

        assert_eq!(table.find(Some("APP.EXAMPLE"), "/api/items").map(|r| r.id.as_str()), Some("api"));
        assert_eq!(table.find(Some("app.example"), "/other").map(|r| r.id.as_str()), Some("host-root"));
        assert_eq!(table.find(Some("other.example"), "/other").map(|r| r.id.as_str()), Some("fallback"));
    }

    #[test]
    fn duplicate_matchers_fail_closed() {
        let error = RouteTable::new(vec![route("a", None, "/"), route("b", None, "/")])
            .err()
            .unwrap_or_else(|| panic!("duplicate matcher must fail"));
        assert!(matches!(error, RoutingError::DuplicateMatcher { .. }));
    }
}

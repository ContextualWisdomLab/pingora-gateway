//! Transport-neutral edge-routing policy.
//!
//! This bounded context decides only which explicitly configured upstream identity a request path
//! selects. It does not own product authentication, authorization, business routing, certificate
//! authority, service discovery, or application state. Route semantics are kept independent from
//! Pingora types so legacy-edge behavior can be characterized before transport activation.

use std::collections::HashSet;

use thiserror::Error;

/// Supported path matching semantics for the shared edge-routing contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteMatch {
    /// Match one request path byte-for-byte.
    Exact(String),
    /// Match any request path beginning with the configured prefix.
    PathPrefix(String),
}

impl RouteMatch {
    fn path(&self) -> &str {
        match self {
            Self::Exact(path) | Self::PathPrefix(path) => path,
        }
    }

    fn matches(&self, request_path: &str) -> bool {
        match self {
            Self::Exact(path) => request_path == path,
            Self::PathPrefix(prefix) => request_path.starts_with(prefix),
        }
    }
}

/// One deterministic edge route from a path matcher to an upstream identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRule {
    /// Stable operator-facing route identity.
    pub name: String,
    /// Explicit precedence. Larger values are evaluated first.
    pub priority: i32,
    /// Request-path matching contract.
    pub matcher: RouteMatch,
    /// Stable upstream identity selected when the route matches.
    pub upstream: String,
}

/// Fail-closed route-contract validation errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RoutePolicyError {
    /// At least one route is required to activate a route table.
    #[error("edge routing requires at least one route")]
    NoRoutes,
    /// Route names are stable identifiers and cannot be empty.
    #[error("route name must not be empty")]
    EmptyRouteName,
    /// Route names must be unique within one activated table.
    #[error("duplicate route name: {route_name}")]
    DuplicateRouteName {
        /// Duplicate route identifier.
        route_name: String,
    },
    /// Equal priorities are rejected because the gateway does not invent undocumented tie-breaks.
    #[error("duplicate route priority: {priority}")]
    DuplicatePriority {
        /// Ambiguous precedence value.
        priority: i32,
    },
    /// Edge paths must be absolute HTTP paths.
    #[error("route {route_name} must use an absolute non-empty path")]
    InvalidPath {
        /// Route whose matcher path is invalid.
        route_name: String,
    },
    /// Upstream identities must be explicit and non-empty.
    #[error("route {route_name} must select a non-empty upstream identity")]
    EmptyUpstream {
        /// Route whose upstream identity is missing.
        route_name: String,
    },
}

/// Validated, deterministic edge-routing table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteTable {
    routes: Vec<RouteRule>,
}

impl RouteTable {
    /// Validates and orders route rules before any request can be routed.
    ///
    /// Equal priorities fail closed. This avoids accidentally depending on container order or on
    /// proxy-specific secondary precedence rules that were not captured in the migration contract.
    pub fn try_new(mut routes: Vec<RouteRule>) -> Result<Self, RoutePolicyError> {
        if routes.is_empty() {
            return Err(RoutePolicyError::NoRoutes);
        }

        let mut names = HashSet::with_capacity(routes.len());
        let mut priorities = HashSet::with_capacity(routes.len());
        for route in &routes {
            let normalized_name = route.name.trim();
            if normalized_name.is_empty() {
                return Err(RoutePolicyError::EmptyRouteName);
            }
            if !names.insert(normalized_name.to_string()) {
                return Err(RoutePolicyError::DuplicateRouteName {
                    route_name: normalized_name.to_string(),
                });
            }
            if !priorities.insert(route.priority) {
                return Err(RoutePolicyError::DuplicatePriority {
                    priority: route.priority,
                });
            }

            let path = route.matcher.path();
            if path.is_empty() || !path.starts_with('/') {
                return Err(RoutePolicyError::InvalidPath {
                    route_name: normalized_name.to_string(),
                });
            }
            if route.upstream.trim().is_empty() {
                return Err(RoutePolicyError::EmptyUpstream {
                    route_name: normalized_name.to_string(),
                });
            }
        }

        routes.sort_by(|left, right| right.priority.cmp(&left.priority));
        Ok(Self { routes })
    }

    /// Selects the upstream identity for one request path using explicit descending priority.
    pub fn select_upstream(&self, request_path: &str) -> Option<&str> {
        self.routes
            .iter()
            .find(|route| route.matcher.matches(request_path))
            .map(|route| route.upstream.as_str())
    }
}

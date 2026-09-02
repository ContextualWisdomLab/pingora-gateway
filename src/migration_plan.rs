//! Transport-neutral composition boundary for one characterized edge migration.
//!
//! A migration plan binds an explicit upstream-authority set to the already validated Edge Routing
//! and HTTP Policy bounded contexts. It deliberately stops before Pingora transport activation: the
//! plan proves that every captured route resolves to an explicitly named upstream and that the
//! response policy is internally valid, without turning the gateway into product auth, service
//! discovery, certificate authority, or business-logic ownership.

use std::collections::HashSet;

use thiserror::Error;

use crate::edge_routing::{RoutePolicyError, RouteRule, RouteTable};
use crate::http_policy::{HeaderPolicyError, ResponseHeaderPolicy, ResponseHeaderRule};

/// Fail-closed errors produced while composing one characterized edge migration plan.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MigrationPlanError {
    /// At least one explicitly approved upstream identity is required.
    #[error("edge migration plan requires at least one upstream identity")]
    NoUpstreams,
    /// Upstream identities are stable names and cannot be empty after trimming whitespace.
    #[error("edge migration upstream name must not be empty")]
    EmptyUpstreamName,
    /// The explicit upstream-authority set cannot contain the same normalized identity twice.
    #[error("duplicate edge migration upstream name: {upstream_name}")]
    DuplicateUpstreamName {
        /// Duplicate normalized upstream identity.
        upstream_name: String,
    },
    /// A characterized route may select only an upstream explicitly admitted by this plan.
    #[error("route {route_name} selects unknown upstream {upstream_name}")]
    UnknownRouteUpstream {
        /// Stable route identity whose target is outside the admitted upstream set.
        route_name: String,
        /// Upstream identity referenced by the route but not admitted by the plan.
        upstream_name: String,
    },
    /// The composed Edge Routing contract is invalid.
    #[error(transparent)]
    RoutePolicy(#[from] RoutePolicyError),
    /// The composed HTTP response policy is invalid.
    #[error(transparent)]
    HeaderPolicy(#[from] HeaderPolicyError),
}

/// Validated transport-neutral contract for one legacy-edge migration candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeMigrationPlan {
    upstreams: HashSet<String>,
    routes: RouteTable,
    response_headers: ResponseHeaderPolicy,
}

impl EdgeMigrationPlan {
    /// Composes explicit upstream authority, Edge Routing, and HTTP Policy before runtime wiring.
    ///
    /// Upstream identities are normalized by trimming surrounding whitespace and duplicates are
    /// rejected. Route targets must then match those normalized identities exactly. This avoids a
    /// characterized route surviving review while pointing at an undeclared service-discovery or
    /// network authority that the migration contract never captured.
    pub fn try_new(
        upstream_names: Vec<String>,
        route_rules: Vec<RouteRule>,
        response_header_rules: Vec<ResponseHeaderRule>,
    ) -> Result<Self, MigrationPlanError> {
        if upstream_names.is_empty() {
            return Err(MigrationPlanError::NoUpstreams);
        }

        let mut upstreams = HashSet::with_capacity(upstream_names.len());
        for upstream_name in upstream_names {
            let normalized_name = upstream_name.trim();
            if normalized_name.is_empty() {
                return Err(MigrationPlanError::EmptyUpstreamName);
            }
            if !upstreams.insert(normalized_name.to_string()) {
                return Err(MigrationPlanError::DuplicateUpstreamName {
                    upstream_name: normalized_name.to_string(),
                });
            }
        }

        let routes = RouteTable::try_new(route_rules.clone())?;
        for route in &route_rules {
            if !upstreams.contains(route.upstream.as_str()) {
                return Err(MigrationPlanError::UnknownRouteUpstream {
                    route_name: route.name.trim().to_string(),
                    upstream_name: route.upstream.clone(),
                });
            }
        }

        let response_headers = ResponseHeaderPolicy::try_new(response_header_rules)?;
        Ok(Self {
            upstreams,
            routes,
            response_headers,
        })
    }

    /// Selects the characterized upstream identity for one request path.
    pub fn select_upstream(&self, request_path: &str) -> Option<&str> {
        self.routes.select_upstream(request_path)
    }

    /// Returns the characterized edge-owned response value for one HTTP field name.
    pub fn response_header_value(&self, name: &str) -> Option<&str> {
        self.response_headers.value_for(name)
    }

    /// Returns every characterized edge-owned response-header mutation in declaration order.
    pub fn response_header_rules(&self) -> &[ResponseHeaderRule] {
        self.response_headers.rules()
    }

    /// Returns the number of explicitly admitted upstream identities in this plan.
    pub fn upstream_count(&self) -> usize {
        self.upstreams.len()
    }

    /// Returns whether an upstream identity is explicitly admitted by this migration plan.
    pub fn contains_upstream(&self, name: &str) -> bool {
        self.upstreams.contains(name.trim())
    }
}

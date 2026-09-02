//! Admin configuration boundary for the characterized `pg-erd-cloud` edge migration.
//!
//! This module activates only the already-characterized route, response-policy and upstream-name
//! contract. Operators may supply concrete listener and upstream transport endpoints plus runtime
//! budgets, but cannot add routes, rename migration authorities, inject product authentication or
//! widen arbitrary per-request network authority through configuration.

use std::net::SocketAddr;

use serde::Deserialize;
use thiserror::Error;

use crate::edge_contract::UpstreamConfig;
use crate::edge_routing::{RouteMatch, RouteRule};
use crate::http_policy::ResponseHeaderRule;
use crate::migration_delivery::{MigrationDeliveryError, MigrationDeliveryPlan};
use crate::migration_plan::{EdgeMigrationPlan, MigrationPlanError};
use crate::migration_proxy::{MigrationGatewayProxy, MigrationGatewayProxyError};
use crate::runtime_isolation::{RuntimeIsolationConfigError, RuntimeIsolationLimits};

/// Version of the bounded `pg-erd-cloud` migration admin configuration.
pub const PG_ERD_MIGRATION_CONFIG_VERSION: u32 = 1;

/// Fail-closed admin configuration for the characterized `pg-erd-cloud` migration runtime.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PgErdMigrationConfig {
    version: u32,
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    max_request_body_bytes: u64,
    max_in_flight_requests: usize,
    upstream_keepalive_pool_size: usize,
    upstreams: Vec<UpstreamConfig>,
}

/// Reasons the bounded migration admin configuration cannot obtain listener authority.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PgErdMigrationConfigError {
    /// YAML could not be decoded into the strict migration-admin contract.
    #[error("pg-erd migration configuration is not valid YAML for the current contract: {0}")]
    Parse(String),
    /// The configuration requests a migration-admin version this binary does not implement.
    #[error("unsupported pg-erd migration configuration version {0}")]
    UnsupportedVersion(u32),
    /// Traffic and metrics endpoints must never compete for the same socket authority.
    #[error("listener and metrics_listener must use distinct socket addresses")]
    ListenerCollision,
    /// A zero keepalive pool would silently change upstream connection-capacity behavior.
    #[error("upstream_keepalive_pool_size must be greater than zero")]
    InvalidUpstreamKeepalivePoolSize,
    /// The fixed route/header migration plan itself failed validation.
    #[error(transparent)]
    Plan(#[from] MigrationPlanError),
    /// Concrete transport authorities do not exactly bind the characterized migration plan.
    #[error(transparent)]
    Delivery(#[from] MigrationDeliveryError),
    /// Runtime-isolation budgets are invalid.
    #[error(transparent)]
    RuntimeIsolation(#[from] RuntimeIsolationConfigError),
    /// The validated delivery plan could not be composed into Pingora callbacks.
    #[error(transparent)]
    Proxy(#[from] MigrationGatewayProxyError),
}

impl PgErdMigrationConfig {
    /// Parses and fully validates the bounded migration configuration before any listener starts.
    pub fn from_yaml(input: &str) -> Result<Self, PgErdMigrationConfigError> {
        let config: Self = serde_yaml::from_str(input)
            .map_err(|error| PgErdMigrationConfigError::Parse(error.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// Returns the downstream listener selected by the operator.
    pub fn listener(&self) -> SocketAddr {
        self.listener
    }

    /// Returns the dedicated low-cardinality metrics listener selected by the operator.
    pub fn metrics_listener(&self) -> SocketAddr {
        self.metrics_listener
    }

    /// Returns the validated Pingora upstream keepalive-pool budget.
    pub fn upstream_keepalive_pool_size(&self) -> usize {
        self.upstream_keepalive_pool_size
    }

    /// Builds the characterized multi-route Pingora callback adapter from explicit transport data.
    ///
    /// The route table and response-header policy are compiled into this bounded migration profile;
    /// configuration can bind only the concrete `backend` and `frontend` transport authorities.
    pub fn build_proxy(&self) -> Result<MigrationGatewayProxy, PgErdMigrationConfigError> {
        let delivery = self.build_delivery()?;
        let limits = RuntimeIsolationLimits::try_new(
            self.max_request_body_bytes,
            self.max_in_flight_requests,
        )?;
        MigrationGatewayProxy::try_new(delivery, limits).map_err(Into::into)
    }

    fn validate(&self) -> Result<(), PgErdMigrationConfigError> {
        if self.version != PG_ERD_MIGRATION_CONFIG_VERSION {
            return Err(PgErdMigrationConfigError::UnsupportedVersion(self.version));
        }
        if self.listener == self.metrics_listener {
            return Err(PgErdMigrationConfigError::ListenerCollision);
        }
        if self.upstream_keepalive_pool_size == 0 {
            return Err(PgErdMigrationConfigError::InvalidUpstreamKeepalivePoolSize);
        }

        RuntimeIsolationLimits::try_new(
            self.max_request_body_bytes,
            self.max_in_flight_requests,
        )?;
        self.build_delivery()?;
        Ok(())
    }

    fn build_delivery(&self) -> Result<MigrationDeliveryPlan, PgErdMigrationConfigError> {
        let plan = pg_erd_migration_plan()?;
        MigrationDeliveryPlan::try_new(plan, self.upstreams.clone()).map_err(Into::into)
    }
}

fn pg_erd_migration_plan() -> Result<EdgeMigrationPlan, MigrationPlanError> {
    EdgeMigrationPlan::try_new(
        vec!["backend".to_string(), "frontend".to_string()],
        vec![
            RouteRule {
                name: "healthz".to_string(),
                priority: 110,
                matcher: RouteMatch::Exact("/healthz".to_string()),
                upstream: "backend".to_string(),
            },
            RouteRule {
                name: "api".to_string(),
                priority: 100,
                matcher: RouteMatch::PathPrefix("/api".to_string()),
                upstream: "backend".to_string(),
            },
            RouteRule {
                name: "frontend".to_string(),
                priority: 1,
                matcher: RouteMatch::PathPrefix("/".to_string()),
                upstream: "frontend".to_string(),
            },
        ],
        vec![
            ResponseHeaderRule {
                name: "X-Content-Type-Options".to_string(),
                value: "nosniff".to_string(),
            },
            ResponseHeaderRule {
                name: "X-Frame-Options".to_string(),
                value: "DENY".to_string(),
            },
            ResponseHeaderRule {
                name: "Referrer-Policy".to_string(),
                value: "no-referrer".to_string(),
            },
            ResponseHeaderRule {
                name: "Permissions-Policy".to_string(),
                value: "geolocation=(), microphone=(), camera=()".to_string(),
            },
        ],
    )
}

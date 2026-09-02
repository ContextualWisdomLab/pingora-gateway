//! Pingora transport binding for one already-characterized edge migration.
//!
//! This delivery boundary turns the stable upstream identities admitted by [`EdgeMigrationPlan`]
//! into concrete, prevalidated Pingora peers. It does not perform service discovery, absorb product
//! routing/authentication logic, or infer network destinations from request data. Every activated
//! peer must come from an explicit [`UpstreamConfig`] whose stable name is already authority in the
//! transport-neutral migration plan.

use std::collections::HashMap;

use pingora::prelude::HttpPeer;
use thiserror::Error;

use crate::edge_contract::UpstreamConfig;
use crate::migration_plan::EdgeMigrationPlan;
use crate::pingora_delivery::{build_peer, PeerBuildError};

/// Fail-closed errors while binding a characterized migration to concrete transport authority.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MigrationDeliveryError {
    /// Every characterized upstream requires exactly one explicit transport binding.
    #[error(
        "edge migration transport authority count mismatch: expected {expected}, received {actual}"
    )]
    UpstreamAuthorityCountMismatch {
        /// Number of upstream identities admitted by the migration plan.
        expected: usize,
        /// Number of concrete upstream configurations supplied for activation.
        actual: usize,
    },
    /// One stable upstream identity cannot resolve to multiple concrete network authorities.
    #[error("duplicate configured migration upstream: {upstream_name}")]
    DuplicateConfiguredUpstream {
        /// Duplicate normalized upstream identity.
        upstream_name: String,
    },
    /// Delivery configuration may bind only upstream identities admitted by the migration plan.
    #[error("configured migration upstream is not admitted by the plan: {upstream_name}")]
    UnknownConfiguredUpstream {
        /// Concrete upstream identity not admitted by the characterized migration.
        upstream_name: String,
    },
    /// A concrete upstream failed validation or Pingora peer construction before activation.
    #[error("unable to activate migration upstream {upstream_name}: {source}")]
    UpstreamActivation {
        /// Stable upstream identity whose transport authority could not be built.
        upstream_name: String,
        /// Fail-closed transport construction error.
        #[source]
        source: PeerBuildError,
    },
}

/// Characterized migration plan with complete, immutable Pingora upstream transport bindings.
#[derive(Debug, Clone)]
pub struct MigrationDeliveryPlan {
    plan: EdgeMigrationPlan,
    peers: HashMap<String, HttpPeer>,
}

impl MigrationDeliveryPlan {
    /// Binds every admitted migration upstream identity to one explicit Pingora peer.
    ///
    /// The number of concrete bindings must equal the plan's admitted authority count. Each
    /// configuration is normalized by its stable name, must already be admitted by the plan, and is
    /// passed through the ordinary fail-closed [`build_peer`] adapter before it can become runtime
    /// network authority. Count equality plus uniqueness plus membership makes the resulting map a
    /// complete bijection over the plan's admitted upstream set. This creates no dynamic lookup or
    /// arbitrary per-request destination path.
    pub fn try_new(
        plan: EdgeMigrationPlan,
        upstreams: Vec<UpstreamConfig>,
    ) -> Result<Self, MigrationDeliveryError> {
        let expected = plan.upstream_count();
        let actual = upstreams.len();
        if actual != expected {
            return Err(MigrationDeliveryError::UpstreamAuthorityCountMismatch {
                expected,
                actual,
            });
        }

        let mut peers = HashMap::with_capacity(actual);
        for upstream in upstreams {
            let upstream_name = upstream.name.trim().to_string();
            if peers.contains_key(&upstream_name) {
                return Err(MigrationDeliveryError::DuplicateConfiguredUpstream {
                    upstream_name,
                });
            }
            if !plan.contains_upstream(&upstream_name) {
                return Err(MigrationDeliveryError::UnknownConfiguredUpstream {
                    upstream_name,
                });
            }

            let peer = build_peer(&upstream).map_err(|source| {
                MigrationDeliveryError::UpstreamActivation {
                    upstream_name: upstream_name.clone(),
                    source,
                }
            })?;
            peers.insert(upstream_name, peer);
        }

        Ok(Self { plan, peers })
    }

    /// Selects the characterized stable upstream identity for one request path.
    pub fn select_upstream_name(&self, request_path: &str) -> Option<&str> {
        self.plan.select_upstream(request_path)
    }

    /// Clones the prevalidated Pingora peer selected by the characterized request path.
    ///
    /// A request that matches no characterized route returns `None`; no fallback destination is
    /// invented. Construction proves every admitted route target has a peer, so indexing by the
    /// plan-selected stable identity preserves that invariant without a second impossible state.
    pub fn build_upstream_peer(&self, request_path: &str) -> Option<HttpPeer> {
        self.plan
            .select_upstream(request_path)
            .map(|upstream_name| self.peers[upstream_name].clone())
    }

    /// Returns the characterized edge-owned response value for one HTTP field name.
    pub fn response_header_value(&self, name: &str) -> Option<&str> {
        self.plan.response_header_value(name)
    }

    /// Returns the number of concrete upstream transport authorities activated for this plan.
    pub fn upstream_count(&self) -> usize {
        self.peers.len()
    }
}

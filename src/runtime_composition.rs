//! Application-layer composition of validated Admin Config into Pingora process configuration.
//!
//! Transport-neutral configuration remains in its owning aggregates. This module is the narrow
//! adapter that maps their runtime-capacity values into Pingora's `ServerConf`. Because both
//! aggregates remain deserializable and the generic aggregate is also programmatically
//! constructible, this public composition boundary revalidates every value that it consumes rather
//! than trusting caller provenance.

use pingora::server::configuration::ServerConf;

use crate::edge_contract::{
    GatewayConfig, GatewayConfigError, MAX_SERVICE_THREADS_PER_SERVICE,
};
use crate::migration_admin::{PgErdMigrationConfig, PgErdMigrationConfigError};
use crate::runtime_policy::build_server_conf_with_service_threads;

/// Builds Pingora process configuration from a generic gateway aggregate after revalidation.
pub fn server_conf_for_gateway(config: &GatewayConfig) -> Result<ServerConf, GatewayConfigError> {
    config.validate()?;
    Ok(build_server_conf_with_service_threads(
        config.upstream_keepalive_pool_size,
        config.service_threads,
    ))
}

/// Builds Pingora process configuration from the characterized pg-erd aggregate after revalidating
/// the runtime-capacity values consumed by `ServerConf`.
///
/// `PgErdMigrationConfig` keeps its fields private, but its public `Deserialize` implementation can
/// still construct a value without invoking `from_yaml`. Rechecking the worker and keepalive values
/// here prevents that representation path from injecting an invalid Pingora runtime topology.
pub fn server_conf_for_pg_erd(
    config: &PgErdMigrationConfig,
) -> Result<ServerConf, PgErdMigrationConfigError> {
    let service_threads = config.service_threads();
    if service_threads == 0 {
        return Err(PgErdMigrationConfigError::InvalidServiceThreads);
    }
    if service_threads > MAX_SERVICE_THREADS_PER_SERVICE {
        return Err(PgErdMigrationConfigError::ServiceThreadsExceedLimit {
            actual: service_threads,
            max: MAX_SERVICE_THREADS_PER_SERVICE,
        });
    }
    if config.upstream_keepalive_pool_size() == 0 {
        return Err(PgErdMigrationConfigError::InvalidUpstreamKeepalivePoolSize);
    }

    Ok(build_server_conf_with_service_threads(
        config.upstream_keepalive_pool_size(),
        service_threads,
    ))
}

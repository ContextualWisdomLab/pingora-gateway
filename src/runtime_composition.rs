//! Application-layer composition of validated Admin Config into Pingora process configuration.
//!
//! Transport-neutral configuration remains in its owning aggregates. This module is the narrow
//! adapter that maps their runtime-capacity values into Pingora's `ServerConf`. Because both
//! aggregates remain deserializable and the generic aggregate is also programmatically
//! constructible, this public composition boundary revalidates every value that it consumes rather
//! than trusting caller provenance.

use pingora::server::configuration::ServerConf;

use crate::edge_contract::{GatewayConfig, GatewayConfigError, MAX_SERVICE_THREADS_PER_SERVICE};
use crate::gateway_proxy::{GatewayProxy, GatewayProxyError};
use crate::migration_admin::{PgErdMigrationConfig, PgErdMigrationConfigError};
use crate::migration_proxy::MigrationGatewayProxy;
use crate::runtime_policy::build_server_conf_with_service_threads;

/// Builds Pingora process configuration from a generic gateway aggregate after revalidation.
pub fn server_conf_for_gateway(config: &GatewayConfig) -> Result<ServerConf, GatewayConfigError> {
    config.validate()?;
    Ok(build_server_conf_with_service_threads(
        config.upstream_keepalive_pool_size,
        config.service_threads,
    ))
}

/// Revalidates and materializes the complete generic runtime before listener authority is granted.
///
/// Keeping process configuration and the transport adapter behind one activation result lets the
/// composition root fail closed once. It also prevents the binary from carrying an impossible
/// second error branch after an already validated configuration has been accepted.
pub fn compose_gateway_runtime(
    config: &GatewayConfig,
) -> Result<(GatewayProxy, ServerConf), GatewayProxyError> {
    let server_conf = server_conf_for_gateway(config)?;
    let proxy = GatewayProxy::try_from_config(config)?;
    Ok((proxy, server_conf))
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

/// Revalidates and materializes the characterized pg-erd runtime before listener activation.
///
/// Runtime-capacity validation happens before trust material and proxy delivery are materialized.
/// Both failure classes return through the same process activation boundary while retaining the
/// canonical `PgErdMigrationConfigError` authority.
pub fn compose_pg_erd_runtime(
    config: &PgErdMigrationConfig,
) -> Result<(MigrationGatewayProxy, ServerConf), PgErdMigrationConfigError> {
    let server_conf = server_conf_for_pg_erd(config)?;
    let proxy = config.build_proxy()?;
    Ok((proxy, server_conf))
}

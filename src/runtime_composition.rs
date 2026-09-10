//! Application-layer composition of validated Admin Config into Pingora process configuration.
//!
//! Transport-neutral configuration remains in its owning aggregates. This module is the narrow
//! adapter that maps their validated runtime-capacity values into Pingora's `ServerConf`, so tests
//! can exercise the same composition path production binaries use.

use pingora::server::configuration::ServerConf;

use crate::edge_contract::GatewayConfig;
use crate::migration_admin::PgErdMigrationConfig;
use crate::runtime_policy::build_server_conf_with_service_threads;

/// Builds Pingora process configuration from the validated generic gateway aggregate.
pub fn server_conf_for_gateway(config: &GatewayConfig) -> ServerConf {
    build_server_conf_with_service_threads(
        config.upstream_keepalive_pool_size,
        config.service_threads,
    )
}

/// Builds Pingora process configuration from the validated characterized pg-erd aggregate.
pub fn server_conf_for_pg_erd(config: &PgErdMigrationConfig) -> ServerConf {
    build_server_conf_with_service_threads(
        config.upstream_keepalive_pool_size(),
        config.service_threads(),
    )
}

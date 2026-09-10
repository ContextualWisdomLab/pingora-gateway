use cwl_pingora_gateway::edge_contract::{
    GatewayConfig, GatewayConfigError, MAX_SERVICE_THREADS_PER_SERVICE,
};
use cwl_pingora_gateway::migration_admin::{PgErdMigrationConfig, PgErdMigrationConfigError};
use cwl_pingora_gateway::runtime_policy::{
    build_server_conf_with_service_threads, V1_DEFAULT_SERVICE_THREADS,
};

fn generic_yaml(service_threads: Option<usize>) -> String {
    let topology = service_threads
        .map(|threads| format!("service_threads: {threads}\n"))
        .unwrap_or_default();
    format!(
        "version: 1\nlistener: 127.0.0.1:6188\nmetrics_listener: 127.0.0.1:6192\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 128\n{topology}upstream_keepalive_pool_size: 32\nupstreams:\n  - name: api\n    address: 127.0.0.1:8080\n    tls: false\n    timeouts:\n      connection_ms: 1250\n      total_connection_ms: 2500\n      read_ms: 7500\n      write_ms: 6500\n      idle_ms: 15000\n"
    )
}

fn pg_erd_yaml(service_threads: Option<usize>) -> String {
    let topology = service_threads
        .map(|threads| format!("service_threads: {threads}\n"))
        .unwrap_or_default();
    format!(
        "version: 1\nlistener: 127.0.0.1:6188\nmetrics_listener: 127.0.0.1:6192\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 128\n{topology}upstream_keepalive_pool_size: 32\nupstreams:\n  - name: backend\n    address: 127.0.0.1:18081\n    tls: false\n    timeouts:\n      connection_ms: 500\n      total_connection_ms: 1000\n      read_ms: 2000\n      write_ms: 2000\n      idle_ms: 5000\n  - name: frontend\n    address: 127.0.0.1:18082\n    tls: false\n    timeouts:\n      connection_ms: 500\n      total_connection_ms: 1000\n      read_ms: 2000\n      write_ms: 2000\n      idle_ms: 5000\n"
    )
}

#[test]
fn generic_admin_config_propagates_explicit_service_threads() {
    let config = GatewayConfig::from_yaml(&generic_yaml(Some(8)))
        .expect("explicit generic worker topology should validate");
    let server_conf = build_server_conf_with_service_threads(
        config.upstream_keepalive_pool_size,
        config.service_threads,
    );

    assert_eq!(config.service_threads, 8);
    assert_eq!(server_conf.threads, 8);
}

#[test]
fn generic_admin_config_preserves_one_worker_compatibility_default() {
    let config = GatewayConfig::from_yaml(&generic_yaml(None))
        .expect("legacy generic config should retain its historical worker topology");

    assert_eq!(config.service_threads, V1_DEFAULT_SERVICE_THREADS);
}

#[test]
fn generic_admin_config_rejects_zero_service_threads() {
    assert_eq!(
        GatewayConfig::from_yaml(&generic_yaml(Some(0))),
        Err(GatewayConfigError::InvalidServiceThreads)
    );
}

#[test]
fn generic_admin_config_rejects_service_threads_above_process_ceiling() {
    let actual = MAX_SERVICE_THREADS_PER_SERVICE + 1;
    assert_eq!(
        GatewayConfig::from_yaml(&generic_yaml(Some(actual))),
        Err(GatewayConfigError::ServiceThreadsExceedLimit {
            actual,
            max: MAX_SERVICE_THREADS_PER_SERVICE,
        })
    );
}

#[test]
fn pg_erd_admin_config_propagates_explicit_service_threads() {
    let config = PgErdMigrationConfig::from_yaml(&pg_erd_yaml(Some(8)))
        .expect("explicit pg-erd worker topology should validate");
    let server_conf = build_server_conf_with_service_threads(
        config.upstream_keepalive_pool_size(),
        config.service_threads(),
    );

    assert_eq!(config.service_threads(), 8);
    assert_eq!(server_conf.threads, 8);
}

#[test]
fn pg_erd_admin_config_preserves_one_worker_compatibility_default() {
    let config = PgErdMigrationConfig::from_yaml(&pg_erd_yaml(None))
        .expect("legacy pg-erd config should retain its historical worker topology");

    assert_eq!(config.service_threads(), V1_DEFAULT_SERVICE_THREADS);
}

#[test]
fn pg_erd_admin_config_rejects_zero_service_threads() {
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&pg_erd_yaml(Some(0))),
        Err(PgErdMigrationConfigError::InvalidServiceThreads)
    );
}

#[test]
fn pg_erd_admin_config_rejects_service_threads_above_process_ceiling() {
    let actual = MAX_SERVICE_THREADS_PER_SERVICE + 1;
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&pg_erd_yaml(Some(actual))),
        Err(PgErdMigrationConfigError::ServiceThreadsExceedLimit {
            actual,
            max: MAX_SERVICE_THREADS_PER_SERVICE,
        })
    );
}

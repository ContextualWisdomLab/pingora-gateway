use std::path::PathBuf;

use cwl_pingora_gateway::edge_contract::{
    GatewayConfig, GatewayConfigError, MAX_SERVICE_THREADS_PER_SERVICE,
};
use cwl_pingora_gateway::gateway_proxy::GatewayProxyError;
use cwl_pingora_gateway::migration_admin::{PgErdMigrationConfig, PgErdMigrationConfigError};
use cwl_pingora_gateway::pingora_delivery::PeerBuildError;
use cwl_pingora_gateway::runtime_composition::{
    compose_gateway_runtime, compose_pg_erd_runtime, server_conf_for_gateway,
    server_conf_for_pg_erd,
};
use cwl_pingora_gateway::runtime_policy::V1_DEFAULT_SERVICE_THREADS;
use pingora::upstreams::peer::Peer;

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
    let server_conf = server_conf_for_gateway(&config)
        .expect("validated generic config should compose a Pingora runtime");

    assert_eq!(config.service_threads, 8);
    assert_eq!(server_conf.threads, 8);
}

#[test]
fn generic_admin_config_preserves_one_worker_compatibility_default() {
    let config = GatewayConfig::from_yaml(&generic_yaml(None))
        .expect("legacy generic config should retain its historical worker topology");

    assert_eq!(config.service_threads, V1_DEFAULT_SERVICE_THREADS);
    assert_eq!(
        server_conf_for_gateway(&config)
            .expect("validated compatibility config should compose")
            .threads,
        V1_DEFAULT_SERVICE_THREADS
    );
}

#[test]
fn generic_admin_config_rejects_zero_service_threads() {
    assert_eq!(
        GatewayConfig::from_yaml(&generic_yaml(Some(0))),
        Err(GatewayConfigError::InvalidServiceThreads)
    );
}

#[test]
fn generic_admin_config_admits_exact_service_thread_ceiling() {
    let config = GatewayConfig::from_yaml(&generic_yaml(Some(MAX_SERVICE_THREADS_PER_SERVICE)))
        .expect("exact generic worker ceiling should remain admissible");

    assert_eq!(
        server_conf_for_gateway(&config)
            .expect("exact generic worker ceiling should compose")
            .threads,
        MAX_SERVICE_THREADS_PER_SERVICE
    );
}

#[test]
fn generic_admin_config_rejects_service_threads_above_data_plane_ceiling() {
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
fn generic_runtime_composition_revalidates_programmatic_topology() {
    let mut config = GatewayConfig::from_yaml(&generic_yaml(Some(8)))
        .expect("initial generic worker topology should validate");
    config.service_threads = 0;

    assert_eq!(
        server_conf_for_gateway(&config).unwrap_err(),
        GatewayConfigError::InvalidServiceThreads
    );
    assert_eq!(
        compose_gateway_runtime(&config).unwrap_err(),
        GatewayProxyError::InvalidConfiguration(GatewayConfigError::InvalidServiceThreads)
    );
}

#[test]
fn generic_runtime_composition_materializes_transport_after_topology_validation() {
    let config = GatewayConfig::from_yaml(&generic_yaml(Some(8)))
        .expect("valid generic runtime should parse and validate");
    let (proxy, server_conf) = compose_gateway_runtime(&config)
        .expect("valid topology and transport should compose together");

    assert_eq!(server_conf.threads, 8);
    assert_eq!(
        proxy.build_upstream_peer().address().to_string(),
        "127.0.0.1:8080"
    );
}

#[test]
fn generic_runtime_composition_preserves_transport_activation_failure() {
    let mut config = GatewayConfig::from_yaml(&generic_yaml(Some(8)))
        .expect("base generic runtime should validate");
    config.upstreams[0].tls = true;
    config.upstreams[0].sni = Some("api.internal.example".to_string());
    config.upstreams[0].trust_bundle_file = Some(PathBuf::from("/definitely/missing/cwl-ca.pem"));

    assert!(matches!(
        compose_gateway_runtime(&config).unwrap_err(),
        GatewayProxyError::UpstreamActivation(PeerBuildError::ReadTrustBundle { .. })
    ));
}

#[test]
fn pg_erd_admin_config_propagates_explicit_service_threads() {
    let config = PgErdMigrationConfig::from_yaml(&pg_erd_yaml(Some(8)))
        .expect("explicit pg-erd worker topology should validate");
    let server_conf = server_conf_for_pg_erd(&config)
        .expect("validated pg-erd config should compose a Pingora runtime");

    assert_eq!(config.service_threads(), 8);
    assert_eq!(server_conf.threads, 8);
}

#[test]
fn pg_erd_admin_config_preserves_one_worker_compatibility_default() {
    let config = PgErdMigrationConfig::from_yaml(&pg_erd_yaml(None))
        .expect("legacy pg-erd config should retain its historical worker topology");

    assert_eq!(config.service_threads(), V1_DEFAULT_SERVICE_THREADS);
    assert_eq!(
        server_conf_for_pg_erd(&config)
            .expect("validated compatibility config should compose")
            .threads,
        V1_DEFAULT_SERVICE_THREADS
    );
}

#[test]
fn pg_erd_admin_config_rejects_zero_service_threads() {
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&pg_erd_yaml(Some(0))),
        Err(PgErdMigrationConfigError::InvalidServiceThreads)
    );
}

#[test]
fn pg_erd_admin_config_admits_exact_service_thread_ceiling() {
    let config =
        PgErdMigrationConfig::from_yaml(&pg_erd_yaml(Some(MAX_SERVICE_THREADS_PER_SERVICE)))
            .expect("exact pg-erd worker ceiling should remain admissible");

    assert_eq!(
        server_conf_for_pg_erd(&config)
            .expect("exact pg-erd worker ceiling should compose")
            .threads,
        MAX_SERVICE_THREADS_PER_SERVICE
    );
}

#[test]
fn pg_erd_admin_config_rejects_service_threads_above_data_plane_ceiling() {
    let actual = MAX_SERVICE_THREADS_PER_SERVICE + 1;
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&pg_erd_yaml(Some(actual))),
        Err(PgErdMigrationConfigError::ServiceThreadsExceedLimit {
            actual,
            max: MAX_SERVICE_THREADS_PER_SERVICE,
        })
    );
}

#[test]
fn pg_erd_runtime_composition_rejects_deserialized_zero_threads() {
    let config: PgErdMigrationConfig = serde_yaml::from_str(&pg_erd_yaml(Some(0)))
        .expect("raw serde construction should demonstrate the validation bypass representation");

    assert_eq!(
        server_conf_for_pg_erd(&config).unwrap_err(),
        PgErdMigrationConfigError::InvalidServiceThreads
    );
    assert_eq!(
        compose_pg_erd_runtime(&config).unwrap_err(),
        PgErdMigrationConfigError::InvalidServiceThreads
    );
}

#[test]
fn pg_erd_runtime_composition_rejects_deserialized_threads_above_ceiling() {
    let actual = MAX_SERVICE_THREADS_PER_SERVICE + 1;
    let config: PgErdMigrationConfig = serde_yaml::from_str(&pg_erd_yaml(Some(actual)))
        .expect("raw serde construction should preserve the invalid worker count for revalidation");

    assert_eq!(
        server_conf_for_pg_erd(&config).unwrap_err(),
        PgErdMigrationConfigError::ServiceThreadsExceedLimit {
            actual,
            max: MAX_SERVICE_THREADS_PER_SERVICE,
        }
    );
}

#[test]
fn pg_erd_runtime_composition_rejects_deserialized_zero_keepalive_pool() {
    let yaml = pg_erd_yaml(Some(8)).replace(
        "upstream_keepalive_pool_size: 32",
        "upstream_keepalive_pool_size: 0",
    );
    let config: PgErdMigrationConfig = serde_yaml::from_str(&yaml)
        .expect("raw serde construction should preserve the invalid keepalive budget");

    assert_eq!(
        server_conf_for_pg_erd(&config).unwrap_err(),
        PgErdMigrationConfigError::InvalidUpstreamKeepalivePoolSize
    );
}

#[test]
fn pg_erd_runtime_composition_materializes_characterized_transport_after_validation() {
    let config = PgErdMigrationConfig::from_yaml(&pg_erd_yaml(Some(8)))
        .expect("valid pg-erd runtime should parse and validate");
    let (_, server_conf) = compose_pg_erd_runtime(&config)
        .expect("valid topology and characterized transport should compose together");

    assert_eq!(server_conf.threads, 8);
}

#[test]
fn pg_erd_runtime_composition_preserves_transport_contract_failure() {
    let yaml = pg_erd_yaml(Some(8)).replace("name: backend", "name: outside-plan");
    let config: PgErdMigrationConfig = serde_yaml::from_str(&yaml)
        .expect("raw serde construction should preserve the invalid transport identity");

    assert_eq!(
        compose_pg_erd_runtime(&config).unwrap_err(),
        PgErdMigrationConfigError::UnknownTransportAuthority {
            upstream_name: "outside-plan".to_string(),
        }
    );
}

#[test]
fn production_composition_and_capacity_evidence_bind_distinct_worker_slots() {
    let generic_root = include_str!("../src/bin/cwl-pingora-gateway.rs");
    let pg_erd_root = include_str!("../src/bin/cwl-pingora-pg-erd-migration.rs");
    let capacity_runner = include_str!("load/run_pg_erd_capacity.sh");

    for composition_root in [generic_root, pg_erd_root] {
        assert_eq!(
            composition_root.matches("server.add_service(").count(),
            2,
            "worker evidence must be revisited if a production process registers a different service count"
        );
        assert_eq!(
            composition_root
                .matches("metrics_service.threads = Some(1);")
                .count(),
            1,
            "metrics service must stay isolated from proxy worker fan-out"
        );
    }

    assert!(capacity_runner.contains("REGISTERED_SERVICE_COUNT=2"));
    assert!(capacity_runner.contains("METRICS_SERVICE_THREADS=1"));
    assert!(capacity_runner.contains("configured_proxy_service_threads=%s\\n"));
    assert!(capacity_runner.contains("configured_metrics_service_threads=%s\\n"));
    assert!(capacity_runner.contains("registered_service_count=%s\\n"));
    assert!(capacity_runner.contains("configured_service_worker_slots=%s\\n"));
    assert!(capacity_runner.contains("$((SERVICE_THREADS + METRICS_SERVICE_THREADS))"));
}

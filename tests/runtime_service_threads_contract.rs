use cwl_pingora_gateway::edge_contract::GatewayConfig;
use cwl_pingora_gateway::migration_admin::PgErdMigrationConfig;

#[test]
fn generic_admin_config_admits_explicit_service_threads() {
    let yaml = r#"
version: 1
listener: 127.0.0.1:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 128
service_threads: 8
upstream_keepalive_pool_size: 32
upstreams:
  - name: api
    address: 127.0.0.1:8080
    tls: false
    timeouts:
      connection_ms: 1250
      total_connection_ms: 2500
      read_ms: 7500
      write_ms: 6500
      idle_ms: 15000
"#;

    assert!(
        GatewayConfig::from_yaml(yaml).is_ok(),
        "the runtime admin boundary must admit an explicit worker topology"
    );
}

#[test]
fn pg_erd_admin_config_admits_explicit_service_threads() {
    let yaml = r#"
version: 1
listener: 127.0.0.1:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 128
service_threads: 8
upstream_keepalive_pool_size: 32
upstreams:
  - name: backend
    address: 127.0.0.1:18081
    tls: false
    timeouts:
      connection_ms: 500
      total_connection_ms: 1000
      read_ms: 2000
      write_ms: 2000
      idle_ms: 5000
  - name: frontend
    address: 127.0.0.1:18082
    tls: false
    timeouts:
      connection_ms: 500
      total_connection_ms: 1000
      read_ms: 2000
      write_ms: 2000
      idle_ms: 5000
"#;

    assert!(
        PgErdMigrationConfig::from_yaml(yaml).is_ok(),
        "the characterized runtime admin boundary must admit an explicit worker topology"
    );
}

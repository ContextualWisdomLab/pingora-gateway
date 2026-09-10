use cwl_pingora_gateway::edge_contract::GatewayConfig;
use cwl_pingora_gateway::migration_admin::PgErdMigrationConfig;

const SHARED_TLS_H2_CONFIG: &str = r#"
version: 2
listener: 127.0.0.1:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 128
service_threads: 4
upstream_keepalive_pool_size: 32
downstream_tls:
  certificate_chain_file: /run/secrets/cwl-edge/tls.crt
  private_key_file: /run/secrets/cwl-edge/tls.key
  alpn: h2_http1
upstreams:
  - name: application
    address: 192.0.2.10:8080
    tls: false
    timeouts:
      connection_ms: 1000
      total_connection_ms: 2000
      read_ms: 5000
      write_ms: 5000
      idle_ms: 10000
"#;

const PG_ERD_TLS_H2_CONFIG: &str = r#"
version: 3
listener: 127.0.0.1:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 128
max_upstream_response_body_ms: 15000
service_threads: 4
upstream_keepalive_pool_size: 32
downstream_tls:
  certificate_chain_file: /run/secrets/cwl-edge/tls.crt
  private_key_file: /run/secrets/cwl-edge/tls.key
  alpn: h2_http1
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

#[test]
fn shared_gateway_requires_a_versioned_positive_downstream_tls_h2_contract() {
    let result = GatewayConfig::from_yaml(SHARED_TLS_H2_CONFIG);
    assert!(
        result.is_ok(),
        "shared gateway must admit an explicit version-2 downstream TLS/H2 contract before TLS listener work; got {result:?}"
    );
}

#[test]
fn pg_erd_migration_requires_a_versioned_positive_downstream_tls_h2_contract() {
    let result = PgErdMigrationConfig::from_yaml(PG_ERD_TLS_H2_CONFIG);
    assert!(
        result.is_ok(),
        "pg-erd migration must admit an explicit version-3 downstream TLS/H2 contract before TLS listener work; got {result:?}"
    );
}

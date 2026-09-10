use cwl_pingora_gateway::downstream_tls::DownstreamTlsConfigError;
use cwl_pingora_gateway::edge_contract::{GatewayConfig, GatewayConfigError};
use cwl_pingora_gateway::migration_admin::{PgErdMigrationConfig, PgErdMigrationConfigError};

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
fn shared_gateway_admits_only_the_opt_in_versioned_downstream_tls_h2_contract() {
    let config = GatewayConfig::from_yaml(SHARED_TLS_H2_CONFIG)
        .expect("shared gateway must admit the explicit version-2 downstream TLS/H2 contract");
    assert!(config.downstream_tls().is_some());

    let legacy_with_tls = SHARED_TLS_H2_CONFIG.replacen("version: 2", "version: 1", 1);
    assert_eq!(
        GatewayConfig::from_yaml(&legacy_with_tls),
        Err(GatewayConfigError::DownstreamTlsRequiresVersion2)
    );

    let incomplete_v2 = SHARED_TLS_H2_CONFIG.replacen(
        "downstream_tls:\n  certificate_chain_file: /run/secrets/cwl-edge/tls.crt\n  private_key_file: /run/secrets/cwl-edge/tls.key\n  alpn: h2_http1\n",
        "",
        1,
    );
    assert_eq!(
        GatewayConfig::from_yaml(&incomplete_v2),
        Err(GatewayConfigError::MissingDownstreamTls)
    );
}

#[test]
fn pg_erd_migration_admits_tls_only_after_the_response_lifetime_contract() {
    let config = PgErdMigrationConfig::from_yaml(PG_ERD_TLS_H2_CONFIG)
        .expect("pg-erd migration must admit the explicit version-3 downstream TLS/H2 contract");
    assert!(config.downstream_tls().is_some());

    let v2_with_tls = PG_ERD_TLS_H2_CONFIG.replacen("version: 3", "version: 2", 1);
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&v2_with_tls),
        Err(PgErdMigrationConfigError::DownstreamTlsRequiresVersion3)
    );

    let missing_lifetime =
        PG_ERD_TLS_H2_CONFIG.replacen("max_upstream_response_body_ms: 15000\n", "", 1);
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&missing_lifetime),
        Err(PgErdMigrationConfigError::MissingUpstreamResponseBodyLifetime)
    );

    let incomplete_v3 = PG_ERD_TLS_H2_CONFIG.replacen(
        "downstream_tls:\n  certificate_chain_file: /run/secrets/cwl-edge/tls.crt\n  private_key_file: /run/secrets/cwl-edge/tls.key\n  alpn: h2_http1\n",
        "",
        1,
    );
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&incomplete_v3),
        Err(PgErdMigrationConfigError::MissingDownstreamTls)
    );
}

#[test]
fn downstream_tls_rejects_relative_or_empty_secret_material_references() {
    let relative_cert = SHARED_TLS_H2_CONFIG.replacen(
        "/run/secrets/cwl-edge/tls.crt",
        "secrets/cwl-edge/tls.crt",
        1,
    );
    assert_eq!(
        GatewayConfig::from_yaml(&relative_cert),
        Err(GatewayConfigError::DownstreamTls(
            DownstreamTlsConfigError::RelativeCertificateChainFile
        ))
    );

    let empty_key = SHARED_TLS_H2_CONFIG.replacen("/run/secrets/cwl-edge/tls.key", "\"   \"", 1);
    assert_eq!(
        GatewayConfig::from_yaml(&empty_key),
        Err(GatewayConfigError::DownstreamTls(
            DownstreamTlsConfigError::EmptyPrivateKeyFile
        ))
    );
}

#[test]
fn downstream_tls_rejects_unrepresented_alpn_modes() {
    let h2c = SHARED_TLS_H2_CONFIG.replacen("alpn: h2_http1", "alpn: h2c", 1);
    assert!(matches!(
        GatewayConfig::from_yaml(&h2c),
        Err(GatewayConfigError::Parse(_))
    ));

    let h3 = PG_ERD_TLS_H2_CONFIG.replacen("alpn: h2_http1", "alpn: h3", 1);
    assert!(matches!(
        PgErdMigrationConfig::from_yaml(&h3),
        Err(PgErdMigrationConfigError::Parse(_))
    ));
}

use cwl_pingora_gateway::edge_contract::{GatewayConfig, GatewayConfigError};
use cwl_pingora_gateway::migration_admin::{PgErdMigrationConfig, PgErdMigrationConfigError};

fn generic_gateway_yaml(listener: &str, metrics_listener: &str) -> String {
    format!(
        r#"version: 1
listener: "{listener}"
metrics_listener: "{metrics_listener}"
max_request_body_bytes: 1048576
max_in_flight_requests: 128
upstream_keepalive_pool_size: 32
upstreams:
  - name: application
    address: 127.0.0.1:8080
    tls: false
    timeouts:
      connection_ms: 100
      total_connection_ms: 200
      read_ms: 300
      write_ms: 400
      idle_ms: 500
"#
    )
}

fn pg_erd_gateway_yaml(backend_address: &str) -> String {
    format!(
        r#"version: 1
listener: 127.0.0.1:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 128
upstream_keepalive_pool_size: 32
upstreams:
  - name: backend
    address: "{backend_address}"
    tls: false
    timeouts:
      connection_ms: 100
      total_connection_ms: 200
      read_ms: 300
      write_ms: 400
      idle_ms: 500
  - name: frontend
    address: 127.0.0.1:3000
    tls: false
    timeouts:
      connection_ms: 100
      total_connection_ms: 200
      read_ms: 300
      write_ms: 400
      idle_ms: 500
"#
    )
}

#[test]
fn generic_gateway_rejects_overlapping_listener_authority() {
    for (listener, metrics_listener) in [
        ("127.0.0.1:6188", "127.0.0.1:6188"),
        ("0.0.0.0:6188", "127.0.0.1:6188"),
        ("127.0.0.1:6188", "0.0.0.0:6188"),
        ("[::1]:6188", "[::1]:6188"),
        ("[::]:6188", "[::1]:6188"),
        ("[::1]:6188", "[::]:6188"),
        ("[::]:6188", "127.0.0.1:6188"),
        ("127.0.0.1:6188", "[::]:6188"),
        ("[::ffff:127.0.0.1]:6188", "127.0.0.1:6188"),
        ("127.0.0.1:6188", "[::ffff:127.0.0.1]:6188"),
        ("[::ffff:127.0.0.1]:6188", "0.0.0.0:6188"),
        ("0.0.0.0:6188", "[::ffff:127.0.0.1]:6188"),
        ("[::ffff:0.0.0.0]:6188", "127.0.0.1:6188"),
        ("127.0.0.1:6188", "[::ffff:0.0.0.0]:6188"),
        ("[::ffff:0.0.0.0]:6188", "[::ffff:127.0.0.1]:6188"),
        ("[::ffff:127.0.0.1]:6188", "[::ffff:0.0.0.0]:6188"),
    ] {
        assert_eq!(
            GatewayConfig::from_yaml(&generic_gateway_yaml(listener, metrics_listener)),
            Err(GatewayConfigError::ListenerCollision),
            "overlapping listener authority must fail closed: {listener} vs {metrics_listener}"
        );
    }
}

#[test]
fn generic_gateway_preserves_distinct_listener_authority() {
    for (listener, metrics_listener) in [
        ("127.0.0.1:6188", "127.0.0.2:6188"),
        ("[::1]:6188", "[::2]:6188"),
        ("[::1]:6188", "127.0.0.1:6188"),
        ("127.0.0.1:6188", "[::1]:6188"),
        ("0.0.0.0:6188", "127.0.0.1:6192"),
    ] {
        assert!(
            GatewayConfig::from_yaml(&generic_gateway_yaml(listener, metrics_listener)).is_ok(),
            "distinct socket authorities must remain configurable: {listener} vs {metrics_listener}"
        );
    }
}

#[test]
fn gateway_profiles_reject_ephemeral_or_wildcard_upstream_authority() {
    assert_eq!(
        GatewayConfig::from_yaml(&generic_gateway_yaml("127.0.0.1:0", "127.0.0.1:6192")),
        Err(GatewayConfigError::ZeroListenerPort)
    );
    assert_eq!(
        GatewayConfig::from_yaml(&generic_gateway_yaml("127.0.0.1:6188", "127.0.0.1:0")),
        Err(GatewayConfigError::ZeroMetricsListenerPort)
    );

    let zero_upstream = generic_gateway_yaml("127.0.0.1:6188", "127.0.0.1:6192")
        .replace("address: 127.0.0.1:8080", "address: 127.0.0.1:0");
    assert_eq!(
        GatewayConfig::from_yaml(&zero_upstream),
        Err(GatewayConfigError::ZeroUpstreamPort {
            upstream_name: "application".to_string(),
        })
    );

    for wildcard_address in ["0.0.0.0:8080", "[::]:8080", "[::ffff:0.0.0.0]:8080"] {
        let wildcard_upstream = generic_gateway_yaml("127.0.0.1:6188", "127.0.0.1:6192").replace(
            "address: 127.0.0.1:8080",
            &format!("address: \"{wildcard_address}\""),
        );
        assert_eq!(
            GatewayConfig::from_yaml(&wildcard_upstream),
            Err(GatewayConfigError::UnspecifiedUpstreamAddress {
                upstream_name: "application".to_string(),
            }),
            "generic profile must reject bind-wildcard upstream authority {wildcard_address}"
        );

        let pg_erd = pg_erd_gateway_yaml(wildcard_address);
        assert_eq!(
            PgErdMigrationConfig::from_yaml(&pg_erd),
            Err(PgErdMigrationConfigError::UpstreamConfiguration(
                GatewayConfigError::UnspecifiedUpstreamAddress {
                    upstream_name: "backend".to_string(),
                }
            )),
            "pg-erd profile must reject bind-wildcard upstream authority {wildcard_address}"
        );
    }
}

use cwl_pingora_gateway::edge_contract::{GatewayConfig, GatewayConfigError};

fn generic_gateway_yaml(listener: &str, metrics_listener: &str) -> String {
    format!(
        r#"version: 1
listener: {listener}
metrics_listener: {metrics_listener}
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
fn generic_gateway_rejects_ephemeral_listener_and_unusable_upstream_ports() {
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
}

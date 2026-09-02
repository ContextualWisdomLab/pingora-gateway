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
fn generic_gateway_rejects_wildcard_listener_aliases_on_the_same_port() {
    for (listener, metrics_listener) in [
        ("0.0.0.0:6188", "127.0.0.1:6188"),
        ("127.0.0.1:6188", "0.0.0.0:6188"),
        ("[::]:6188", "[::1]:6188"),
        ("[::]:6188", "127.0.0.1:6188"),
    ] {
        assert_eq!(
            GatewayConfig::from_yaml(&generic_gateway_yaml(listener, metrics_listener)),
            Err(GatewayConfigError::ListenerCollision),
            "wildcard listener authority must not overlap metrics authority: {listener} vs {metrics_listener}"
        );
    }

    assert!(
        GatewayConfig::from_yaml(&generic_gateway_yaml(
            "127.0.0.1:6188",
            "127.0.0.2:6188"
        ))
        .is_ok(),
        "distinct concrete IP authorities on the same port must remain configurable"
    );
}

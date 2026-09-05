use cwl_pingora_gateway::edge_contract::{GatewayConfig, GatewayConfigError};

fn with_runtime_limits(yaml: &str) -> String {
    yaml.replacen(
        "listener: 127.0.0.1:6188",
        "listener: 127.0.0.1:6188\nmetrics_listener: 127.0.0.1:6192\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 128\nupstream_keepalive_pool_size: 32",
        1,
    )
}

#[test]
fn parses_minimal_https_upstream_contract() {
    let yaml = with_runtime_limits(
        r#"
version: 1
listener: 127.0.0.1:6188
upstreams:
  - name: api
    address: 127.0.0.1:8080
    tls: true
    sni: api.internal.example
    timeouts:
      connection_ms: 1250
      total_connection_ms: 2500
      read_ms: 7500
      write_ms: 6500
      idle_ms: 15000
"#,
    );

    let config = GatewayConfig::from_yaml(&yaml).expect("valid gateway contract");

    assert_eq!(config.version, 1);
    assert_eq!(config.listener.to_string(), "127.0.0.1:6188");
    assert_eq!(config.metrics_listener.to_string(), "127.0.0.1:6192");
    assert_eq!(config.max_request_body_bytes, 1_048_576);
    assert_eq!(config.max_in_flight_requests, 128);
    assert_eq!(config.upstream_keepalive_pool_size, 32);
    assert_eq!(config.upstreams.len(), 1);
    assert_eq!(config.upstreams[0].name, "api");
    assert!(config.upstreams[0].tls);
    assert_eq!(
        config.upstreams[0].sni.as_deref(),
        Some("api.internal.example")
    );
    assert_eq!(config.upstreams[0].timeouts.connection_ms, 1_250);
}

#[test]
fn admits_explicit_connection_and_backpressure_budgets() {
    let yaml = r#"
version: 1
listener: 127.0.0.1:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 128
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
        "v1 must admit explicit downstream concurrency and upstream keepalive budgets"
    );
}

#[test]
fn rejects_malformed_yaml_before_network_authority() {
    let error = GatewayConfig::from_yaml("version: [not-a-number]").unwrap_err();
    assert!(matches!(error, GatewayConfigError::Parse(_)));
}

#[test]
fn rejects_unknown_contract_version() {
    let yaml = with_runtime_limits(
        r#"
version: 2
listener: 127.0.0.1:6188
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
"#,
    );

    assert_eq!(
        GatewayConfig::from_yaml(&yaml),
        Err(GatewayConfigError::UnsupportedVersion(2))
    );
}

#[test]
fn rejects_listener_collision() {
    let yaml = r#"
version: 1
listener: 127.0.0.1:6188
metrics_listener: 127.0.0.1:6188
max_request_body_bytes: 1048576
max_in_flight_requests: 128
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

    assert_eq!(
        GatewayConfig::from_yaml(yaml),
        Err(GatewayConfigError::ListenerCollision)
    );
}

#[test]
fn rejects_zero_request_body_limit() {
    let yaml = r#"
version: 1
listener: 127.0.0.1:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 0
max_in_flight_requests: 128
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

    assert_eq!(
        GatewayConfig::from_yaml(yaml),
        Err(GatewayConfigError::InvalidRequestBodyLimit)
    );
}

#[test]
fn rejects_zero_in_flight_request_limit() {
    let yaml = r#"
version: 1
listener: 127.0.0.1:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 0
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

    assert_eq!(
        GatewayConfig::from_yaml(yaml),
        Err(GatewayConfigError::InvalidInFlightRequestLimit)
    );
}

#[test]
fn rejects_zero_upstream_keepalive_pool_size() {
    let yaml = r#"
version: 1
listener: 127.0.0.1:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 128
upstream_keepalive_pool_size: 0
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

    assert_eq!(
        GatewayConfig::from_yaml(yaml),
        Err(GatewayConfigError::InvalidUpstreamKeepalivePoolSize)
    );
}

#[test]
fn rejects_tls_upstream_without_sni() {
    let yaml = with_runtime_limits(
        r#"
version: 1
listener: 127.0.0.1:6188
upstreams:
  - name: api
    address: 127.0.0.1:8443
    tls: true
    timeouts:
      connection_ms: 1250
      total_connection_ms: 2500
      read_ms: 7500
      write_ms: 6500
      idle_ms: 15000
"#,
    );

    assert_eq!(
        GatewayConfig::from_yaml(&yaml),
        Err(GatewayConfigError::MissingTlsServerName {
            upstream_name: "api".to_string()
        })
    );
}

#[test]
fn rejects_empty_tls_server_name() {
    let yaml = with_runtime_limits(
        r#"
version: 1
listener: 127.0.0.1:6188
upstreams:
  - name: api
    address: 127.0.0.1:8443
    tls: true
    sni: "   "
    timeouts:
      connection_ms: 1250
      total_connection_ms: 2500
      read_ms: 7500
      write_ms: 6500
      idle_ms: 15000
"#,
    );

    assert_eq!(
        GatewayConfig::from_yaml(&yaml),
        Err(GatewayConfigError::EmptyTlsServerName {
            upstream_name: "api".to_string()
        })
    );
}

#[test]
fn rejects_sni_on_cleartext_upstream() {
    let yaml = with_runtime_limits(
        r#"
version: 1
listener: 127.0.0.1:6188
upstreams:
  - name: api
    address: 127.0.0.1:8080
    tls: false
    sni: api.internal.example
    timeouts:
      connection_ms: 1250
      total_connection_ms: 2500
      read_ms: 7500
      write_ms: 6500
      idle_ms: 15000
"#,
    );

    assert_eq!(
        GatewayConfig::from_yaml(&yaml),
        Err(GatewayConfigError::UnexpectedTlsServerName {
            upstream_name: "api".to_string()
        })
    );
}

#[test]
fn rejects_empty_upstream_name() {
    let yaml = with_runtime_limits(
        r#"
version: 1
listener: 127.0.0.1:6188
upstreams:
  - name: "   "
    address: 127.0.0.1:8080
    tls: false
    timeouts:
      connection_ms: 1250
      total_connection_ms: 2500
      read_ms: 7500
      write_ms: 6500
      idle_ms: 15000
"#,
    );

    assert_eq!(
        GatewayConfig::from_yaml(&yaml),
        Err(GatewayConfigError::EmptyUpstreamName)
    );
}

#[test]
fn rejects_empty_upstream_set() {
    let yaml = r#"
version: 1
listener: 127.0.0.1:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 128
upstream_keepalive_pool_size: 32
upstreams: []
"#;

    assert_eq!(
        GatewayConfig::from_yaml(yaml),
        Err(GatewayConfigError::NoUpstreams)
    );
}

#[test]
fn rejects_multiple_upstreams_in_version_one() {
    let yaml = with_runtime_limits(
        r#"
version: 1
listener: 127.0.0.1:6188
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
  - name: other
    address: 127.0.0.1:8081
    tls: false
    timeouts:
      connection_ms: 1250
      total_connection_ms: 2500
      read_ms: 7500
      write_ms: 6500
      idle_ms: 15000
"#,
    );

    assert_eq!(
        GatewayConfig::from_yaml(&yaml),
        Err(GatewayConfigError::UnsupportedUpstreamCount { actual: 2 })
    );
}

#[test]
fn rejects_duplicate_upstream_names() {
    let yaml = with_runtime_limits(
        r#"
version: 1
listener: 127.0.0.1:6188
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
  - name: api
    address: 127.0.0.1:8081
    tls: false
    timeouts:
      connection_ms: 1250
      total_connection_ms: 2500
      read_ms: 7500
      write_ms: 6500
      idle_ms: 15000
"#,
    );

    assert_eq!(
        GatewayConfig::from_yaml(&yaml),
        Err(GatewayConfigError::DuplicateUpstreamName {
            upstream_name: "api".to_string()
        })
    );
}

#[test]
fn rejects_zero_timeout_budget() {
    let yaml = with_runtime_limits(
        r#"
version: 1
listener: 127.0.0.1:6188
upstreams:
  - name: api
    address: 127.0.0.1:8080
    tls: false
    timeouts:
      connection_ms: 0
      total_connection_ms: 2500
      read_ms: 7500
      write_ms: 6500
      idle_ms: 15000
"#,
    );

    assert_eq!(
        GatewayConfig::from_yaml(&yaml),
        Err(GatewayConfigError::InvalidTimeoutBudget {
            upstream_name: "api".to_string(),
            timeout_name: "connection_ms",
        })
    );
}

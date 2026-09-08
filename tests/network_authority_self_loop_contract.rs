//! Executable acceptance for separating gateway-owned listener authority from upstream authority.
//!
//! The fixtures exercise both configuration roots and compiled startup so recursive self-proxying
//! or application routing into the metrics socket cannot become network authority.

use std::io::Write;
use std::process::Command;

use cwl_pingora_gateway::edge_contract::{GatewayConfig, GatewayConfigError};
use cwl_pingora_gateway::migration_admin::{PgErdMigrationConfig, PgErdMigrationConfigError};
use tempfile::NamedTempFile;

/// Builds one generic v1 configuration with explicit listener, metrics, and upstream authorities.
fn generic_yaml(listener: &str, metrics_listener: &str, upstream: &str) -> String {
    format!(
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 128\nupstream_keepalive_pool_size: 32\nupstreams:\n  - name: api\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1250\n      total_connection_ms: 2500\n      read_ms: 7500\n      write_ms: 6500\n      idle_ms: 15000\n"
    )
}

/// Builds the fixed pg-erd profile while varying only the two characterized upstream sockets.
fn pg_erd_yaml(listener: &str, metrics_listener: &str, backend: &str, frontend: &str) -> String {
    format!(
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 128\nupstream_keepalive_pool_size: 64\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 100\n      total_connection_ms: 200\n      read_ms: 300\n      write_ms: 400\n      idle_ms: 500\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 100\n      total_connection_ms: 200\n      read_ms: 300\n      write_ms: 400\n      idle_ms: 500\n"
    )
}

/// Persists a configuration fixture so compiled binaries traverse their real startup boundary.
fn config_file(yaml: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    file.write_all(yaml.as_bytes())
        .expect("configuration fixture should be written");
    file
}

/// Generic v1 rejects exact, wildcard, and conservative dual-stack aliases of owned listeners.
#[test]
fn generic_config_rejects_recursive_gateway_authority() {
    for (listener, metrics, upstream, expected) in [
        (
            "127.0.0.1:6188",
            "127.0.0.1:6192",
            "127.0.0.1:6188",
            GatewayConfigError::UpstreamListenerCollision {
                upstream_name: "api".to_string(),
            },
        ),
        (
            "127.0.0.1:6188",
            "127.0.0.1:6192",
            "127.0.0.1:6192",
            GatewayConfigError::UpstreamMetricsListenerCollision {
                upstream_name: "api".to_string(),
            },
        ),
        (
            "0.0.0.0:6188",
            "127.0.0.1:6192",
            "127.0.0.1:6188",
            GatewayConfigError::UpstreamListenerCollision {
                upstream_name: "api".to_string(),
            },
        ),
        (
            "127.0.0.1:6188",
            "[::]:6192",
            "127.0.0.1:6192",
            GatewayConfigError::UpstreamMetricsListenerCollision {
                upstream_name: "api".to_string(),
            },
        ),
    ] {
        assert_eq!(
            GatewayConfig::from_yaml(&generic_yaml(listener, metrics, upstream)),
            Err(expected),
            "upstream alias must fail closed for {listener} / {metrics} / {upstream}"
        );
    }
}

/// Same-port distinct concrete IP authorities remain valid because they do not alias a listener.
#[test]
fn generic_config_preserves_distinct_concrete_same_port_authority() {
    GatewayConfig::from_yaml(&generic_yaml(
        "127.0.0.1:6188",
        "127.0.0.1:6192",
        "127.0.0.2:6188",
    ))
    .expect("distinct concrete IP authority on the same port must remain configurable");
}

/// The bounded migration profile applies the shared invariant after fixed upstream identity checks.
#[test]
fn pg_erd_config_rejects_gateway_owned_socket_as_transport_authority() {
    for (listener, metrics, backend, frontend, expected) in [
        (
            "127.0.0.1:8080",
            "127.0.0.1:9090",
            "127.0.0.1:8080",
            "127.0.0.1:3000",
            GatewayConfigError::UpstreamListenerCollision {
                upstream_name: "backend".to_string(),
            },
        ),
        (
            "127.0.0.1:8080",
            "127.0.0.1:9090",
            "127.0.0.1:8000",
            "127.0.0.1:9090",
            GatewayConfigError::UpstreamMetricsListenerCollision {
                upstream_name: "frontend".to_string(),
            },
        ),
        (
            "0.0.0.0:8080",
            "127.0.0.1:9090",
            "127.0.0.1:8080",
            "127.0.0.1:3000",
            GatewayConfigError::UpstreamListenerCollision {
                upstream_name: "backend".to_string(),
            },
        ),
    ] {
        assert_eq!(
            PgErdMigrationConfig::from_yaml(&pg_erd_yaml(listener, metrics, backend, frontend)),
            Err(PgErdMigrationConfigError::UpstreamConfiguration(expected)),
            "characterized upstream alias must fail closed before activation"
        );
    }
}

/// Direct deserialization cannot bypass network-authority revalidation at the public build boundary.
#[test]
fn pg_erd_build_proxy_revalidates_recursive_authority() {
    let yaml = pg_erd_yaml(
        "127.0.0.1:8080",
        "127.0.0.1:9090",
        "127.0.0.1:8080",
        "127.0.0.1:3000",
    );
    let config: PgErdMigrationConfig =
        serde_yaml::from_str(&yaml).expect("direct deserialization should construct the boundary");
    assert!(matches!(
        config.build_proxy(),
        Err(PgErdMigrationConfigError::UpstreamConfiguration(
            GatewayConfigError::UpstreamListenerCollision { .. }
        ))
    ));
}

/// The generic compiled binary rejects a recursive upstream before a Pingora listener can open.
#[test]
fn generic_binary_fails_closed_before_recursive_listener_activation() {
    let file = config_file(&generic_yaml(
        "127.0.0.1:6188",
        "127.0.0.1:6192",
        "127.0.0.1:6188",
    ));
    let output = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
        .args([
            "--config",
            file.path().to_str().expect("temporary path must be UTF-8"),
        ])
        .output()
        .expect("compiled generic gateway should be executable");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("upstream api socket authority must not overlap listener"),
        "startup error should identify rejected recursive authority; got {stderr:?}"
    );
}

/// The pg-erd compiled binary rejects a recursive backend before a Pingora listener can open.
#[test]
fn pg_erd_binary_fails_closed_before_recursive_listener_activation() {
    let file = config_file(&pg_erd_yaml(
        "127.0.0.1:18080",
        "127.0.0.1:18082",
        "127.0.0.1:18080",
        "127.0.0.1:18083",
    ));
    let output = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"))
        .args([
            "--config",
            file.path().to_str().expect("temporary path must be UTF-8"),
        ])
        .output()
        .expect("compiled pg-erd gateway should be executable");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("upstream backend socket authority must not overlap listener"),
        "startup error should identify rejected backend authority; got {stderr:?}"
    );
}

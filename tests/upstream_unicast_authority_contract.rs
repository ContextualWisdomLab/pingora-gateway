//! Acceptance for rejecting IP destinations that cannot be remote TCP peer authority.
//!
//! RFC 9293 requires an active TCP open to reject invalid remote addresses such as broadcast or
//! multicast destinations. The gateway therefore fails these values at Admin Config validation
//! instead of delegating deterministic misconfiguration to the runtime connect path.

use cwl_pingora_gateway::edge_contract::{GatewayConfig, GatewayConfigError};
use cwl_pingora_gateway::migration_admin::{PgErdMigrationConfig, PgErdMigrationConfigError};

fn generic_yaml(upstream: &str) -> String {
    format!(
        "version: 1\nlistener: \"127.0.0.1:6188\"\nmetrics_listener: \"127.0.0.1:6192\"\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 128\nupstream_keepalive_pool_size: 32\nupstreams:\n  - name: api\n    address: \"{upstream}\"\n    tls: false\n    timeouts:\n      connection_ms: 1250\n      total_connection_ms: 2500\n      read_ms: 7500\n      write_ms: 6500\n      idle_ms: 15000\n"
    )
}

fn pg_erd_yaml(backend: &str) -> String {
    format!(
        "version: 1\nlistener: \"127.0.0.1:8080\"\nmetrics_listener: \"127.0.0.1:9090\"\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 128\nupstream_keepalive_pool_size: 64\nupstreams:\n  - name: backend\n    address: \"{backend}\"\n    tls: false\n    timeouts:\n      connection_ms: 100\n      total_connection_ms: 200\n      read_ms: 300\n      write_ms: 400\n      idle_ms: 500\n  - name: frontend\n    address: \"127.0.0.1:3000\"\n    tls: false\n    timeouts:\n      connection_ms: 100\n      total_connection_ms: 200\n      read_ms: 300\n      write_ms: 400\n      idle_ms: 500\n"
    )
}

#[test]
fn generic_config_rejects_non_unicast_tcp_destinations() {
    for upstream in [
        "224.0.0.1:7000",
        "255.255.255.255:7000",
        "[ff02::1]:7000",
        "[::ffff:224.0.0.1]:7000",
        "[::ffff:255.255.255.255]:7000",
    ] {
        assert_eq!(
            GatewayConfig::from_yaml(&generic_yaml(upstream)),
            Err(GatewayConfigError::NonUnicastUpstreamAddress {
                upstream_name: "api".to_string(),
            }),
            "broadcast or multicast TCP upstream authority must fail closed: {upstream}"
        );
    }
}

#[test]
fn pg_erd_config_rejects_non_unicast_tcp_destinations() {
    for backend in [
        "224.0.0.1:7000",
        "255.255.255.255:7000",
        "[ff02::1]:7000",
        "[::ffff:224.0.0.1]:7000",
        "[::ffff:255.255.255.255]:7000",
    ] {
        assert_eq!(
            PgErdMigrationConfig::from_yaml(&pg_erd_yaml(backend)),
            Err(PgErdMigrationConfigError::UpstreamConfiguration(
                GatewayConfigError::NonUnicastUpstreamAddress {
                    upstream_name: "backend".to_string(),
                }
            )),
            "pg-erd must reject broadcast or multicast TCP upstream authority: {backend}"
        );
    }
}

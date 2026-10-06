//! Generic v1 concrete endpoint admission, from Issue #26 and the narrow-repair residual gap.
//!
//! These tests exercise only the strict configuration contract. They never open listeners or send
//! traffic, so an invalid authority is proven to fail before network activation.

use cwl_pingora_gateway::edge_contract::GatewayConfig;

/// Constructs an otherwise-valid single-upstream configuration without acquiring sockets.
fn source(listener: &str, metrics: &str, upstream: &str) -> String {
    format!(
        "version: 1\nlistener: \"{listener}\"\nmetrics_listener: \"{metrics}\"\nmax_request_body_bytes: 8\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: application\n    address: \"{upstream}\"\n    tls: false\n    timeouts:\n      connection_ms: 100\n      total_connection_ms: 200\n      read_ms: 300\n      write_ms: 400\n      idle_ms: 500\n"
    )
}

/// Returns the public diagnostic produced by the real YAML contract, or `None` when admitted.
fn rejection(listener: &str, metrics: &str, upstream: &str) -> Option<String> {
    GatewayConfig::from_yaml(&source(listener, metrics, upstream))
        .err()
        .map(|error| error.to_string())
}

#[test]
fn rejects_zero_port_gateway_listeners() {
    for listener in ["127.0.0.1:0", "0.0.0.0:0", "[::]:0"] {
        assert_eq!(
            rejection(listener, "127.0.0.1:6192", "127.0.0.1:8080"),
            Some("listener must use a non-zero port".to_string()),
            "{listener}"
        );
    }
    for metrics in ["127.0.0.1:0", "[::1]:0"] {
        assert_eq!(
            rejection("127.0.0.1:6188", metrics, "127.0.0.1:8080"),
            Some("metrics_listener must use a non-zero port".to_string()),
            "{metrics}"
        );
    }
}

#[test]
fn rejects_zero_port_upstream() {
    for upstream in ["127.0.0.1:0", "[::1]:0", "192.0.2.10:0"] {
        assert_eq!(
            rejection("127.0.0.1:6188", "127.0.0.1:6192", upstream),
            Some("upstream application must use a non-zero port".to_string()),
            "{upstream}"
        );
    }
}

#[test]
fn rejects_unspecified_upstream_address() {
    // Distinct ports isolate the bind-wildcard defect from listener self-routing.
    for upstream in ["0.0.0.0:8080", "[::]:8080", "[::ffff:0.0.0.0]:8080"] {
        assert_eq!(
            rejection("127.0.0.1:6188", "127.0.0.1:6192", upstream),
            Some("upstream application must not use an unspecified address".to_string()),
            "{upstream}"
        );
    }
}

#[test]
fn rejects_listener_to_metrics_alias_overlap() {
    for (listener, metrics) in [
        ("0.0.0.0:6188", "127.0.0.1:6188"),
        ("127.0.0.1:6188", "0.0.0.0:6188"),
        ("[::]:6188", "127.0.0.1:6188"),
        ("[::ffff:127.0.0.1]:6188", "127.0.0.1:6188"),
        ("[::]:6188", "[::1]:6188"),
        ("0.0.0.0:6188", "[::]:6188"),
        ("[::ffff:0.0.0.0]:6188", "[::ffff:127.0.0.1]:6188"),
    ] {
        assert_eq!(
            rejection(listener, metrics, "127.0.0.1:8080"),
            Some("listener and metrics_listener socket authorities must not overlap".to_string()),
            "{listener} / {metrics}"
        );
    }
}

#[test]
fn blank_upstream_name_takes_precedence_over_self_routing() {
    // Moving self-routing ahead of per-upstream validation must not hide an invalid identity.
    for upstream in ["127.0.0.1:6188", "0.0.0.0:6188", "127.0.0.1:6192"] {
        let config = source("127.0.0.1:6188", "127.0.0.1:6192", upstream)
            .replace("name: application", "name: \"   \"");
        assert_eq!(
            GatewayConfig::from_yaml(&config)
                .err()
                .map(|error| error.to_string()),
            Some("upstream name must not be empty".to_string()),
            "{upstream}"
        );
    }
}

#[test]
fn upstream_validate_rejects_blank_name_when_called_directly() {
    // `UpstreamConfig::validate` is public and the delivery adapter calls it without the
    // enclosing aggregate, so the identity guard must hold on that path as well.
    let config = GatewayConfig::from_yaml(&source(
        "127.0.0.1:6188",
        "127.0.0.1:6192",
        "127.0.0.1:8080",
    ))
    .expect("baseline configuration should be valid");
    let mut upstream = config.upstreams[0].clone();
    upstream.name = "   ".to_string();
    assert_eq!(
        upstream.validate().err().map(|error| error.to_string()),
        Some("upstream name must not be empty".to_string())
    );
}

#[test]
fn preserves_concrete_distinct_endpoints() {
    for (listener, metrics, upstream) in [
        ("0.0.0.0:6188", "127.0.0.1:6192", "127.0.0.1:8080"),
        ("[::]:6188", "[::1]:6192", "[::1]:8080"),
        ("127.0.0.1:6188", "127.0.0.2:6188", "192.0.2.10:443"),
        ("[::1]:6188", "127.0.0.1:6188", "10.0.0.5:6188"),
    ] {
        assert_eq!(
            rejection(listener, metrics, upstream),
            None,
            "{listener} / {metrics} / {upstream}"
        );
    }
}
